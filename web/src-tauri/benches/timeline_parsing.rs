use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::io::Write;
use tempfile::NamedTempFile;

// We need to test the parsing performance, but since the functions are private,
// we'll simulate the parsing logic here for benchmarking purposes.

fn create_test_jsonl(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

fn create_small_session_file() -> NamedTempFile {
    let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message"}}
{"timestamp":"2025-01-10T09:30:00+08:00","message":{"role":"assistant","content":"Response"}}
{"timestamp":"2025-01-10T10:00:00+08:00"}"#;
    create_test_jsonl(content)
}

fn create_large_session_file(line_count: usize) -> NamedTempFile {
    let mut content = String::new();

    content.push_str(r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message for the session"}}"#);
    content.push('\n');

    for i in 0..line_count {
        content.push_str(&format!(
            r#"{{"timestamp":"2025-01-10T{:02}:{:02}:00+08:00","message":{{"role":"assistant","content":"Response line {} with some padding text to make this longer and reach the size threshold we need for testing"}}}}"#,
            9 + (i / 60) % 8,
            i % 60,
            i
        ));
        content.push('\n');
    }

    content.push_str(r#"{"timestamp":"2025-01-10T17:00:00+08:00","message":{"role":"assistant","content":"Final response"}}"#);

    create_test_jsonl(&content)
}

/// Simulates full file parsing (reading every line)
fn parse_full(path: &std::path::PathBuf) -> Option<(String, String)> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                if first_ts.is_none() {
                    first_ts = Some(ts.to_string());
                }
                last_ts = Some(ts.to_string());
            }
        }
    }

    match (first_ts, last_ts) {
        (Some(f), Some(l)) => Some((f, l)),
        _ => None,
    }
}

/// Simulates fast file parsing (reading only head and tail)
fn parse_fast(path: &std::path::PathBuf) -> Option<(String, String)> {
    use std::io::{BufRead, BufReader, Seek, SeekFrom};

    let file = std::fs::File::open(path).ok()?;
    let file_size = file.metadata().ok()?.len();

    if file_size < 50_000 {
        return parse_full(path);
    }

    let mut reader = BufReader::new(file);

    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;

    // Read first 20 lines
    let mut lines_read = 0;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                    if first_ts.is_none() {
                        if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                            first_ts = Some(ts.to_string());
                        }
                    }
                }
                lines_read += 1;
                if lines_read >= 20 && first_ts.is_some() {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    // Read last 32KB
    let tail_size: u64 = 32_000.min(file_size);
    let seek_pos = file_size.saturating_sub(tail_size);

    if reader.seek(SeekFrom::Start(seek_pos)).is_ok() {
        if seek_pos > 0 {
            let mut skip_line = String::new();
            let _ = reader.read_line(&mut skip_line);
        }

        for line in reader.lines().flatten() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                    last_ts = Some(ts.to_string());
                }
            }
        }
    }

    let last_ts = last_ts.or_else(|| first_ts.clone());
    match (first_ts, last_ts) {
        (Some(f), Some(l)) => Some((f, l)),
        _ => None,
    }
}

fn benchmark_small_file(c: &mut Criterion) {
    let file = create_small_session_file();
    let path = file.path().to_path_buf();

    c.bench_function("parse_small_file_full", |b| {
        b.iter(|| parse_full(black_box(&path)))
    });

    c.bench_function("parse_small_file_fast", |b| {
        b.iter(|| parse_fast(black_box(&path)))
    });
}

fn benchmark_large_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_file_parsing");

    for line_count in [500, 1000, 2000].iter() {
        let file = create_large_session_file(*line_count);
        let path = file.path().to_path_buf();
        let file_size = std::fs::metadata(&path).unwrap().len();

        group.bench_with_input(
            BenchmarkId::new("full", format!("{}lines_{}KB", line_count, file_size / 1024)),
            &path,
            |b, path| b.iter(|| parse_full(black_box(path))),
        );

        group.bench_with_input(
            BenchmarkId::new("fast", format!("{}lines_{}KB", line_count, file_size / 1024)),
            &path,
            |b, path| b.iter(|| parse_fast(black_box(path))),
        );
    }

    group.finish();
}

fn benchmark_multiple_files(c: &mut Criterion) {
    // Simulate scanning multiple project directories
    let files: Vec<_> = (0..20)
        .map(|i| {
            let content = format!(
                r#"{{"timestamp":"2025-01-10T{:02}:00:00+08:00","cwd":"/home/user/project{}","message":{{"role":"user","content":"Meaningful message for session {}"}}}}
{{"timestamp":"2025-01-10T{:02}:30:00+08:00"}}"#,
                9 + i % 8,
                i,
                i,
                9 + i % 8
            );
            create_test_jsonl(&content)
        })
        .collect();

    let paths: Vec<_> = files.iter().map(|f| f.path().to_path_buf()).collect();

    c.bench_function("parse_20_small_files_sequential", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for path in &paths {
                if let Some(r) = parse_fast(black_box(path)) {
                    results.push(r);
                }
            }
            results
        })
    });

    c.bench_function("parse_20_small_files_parallel", |b| {
        b.iter(|| {
            use std::sync::Mutex;
            use std::thread;

            let results: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
            let paths_ref = &paths;

            thread::scope(|s| {
                let chunk_size = (paths_ref.len() / 4).max(1);
                for chunk in paths_ref.chunks(chunk_size) {
                    let results_ref = &results;
                    s.spawn(move || {
                        let mut local = Vec::new();
                        for path in chunk {
                            if let Some(r) = parse_fast(black_box(path)) {
                                local.push(r);
                            }
                        }
                        results_ref.lock().unwrap().extend(local);
                    });
                }
            });

            results.into_inner().unwrap()
        })
    });
}

criterion_group!(
    benches,
    benchmark_small_file,
    benchmark_large_file,
    benchmark_multiple_files,
);
criterion_main!(benches);
