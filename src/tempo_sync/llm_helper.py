"""
LLM 輔助模組 - 支援多種 LLM 後端

支援:
- Anthropic Claude
- OpenAI GPT
- Google Gemini
- Ollama (本地)
"""

import os
from typing import Optional, Literal
from dataclasses import dataclass


LLMProvider = Literal["anthropic", "openai", "gemini", "ollama", "openai-compatible"]


@dataclass
class LLMConfig:
    """LLM 配置"""
    provider: LLMProvider = "ollama"
    model: str = ""  # 若為空則使用預設
    api_key: str = ""
    ollama_host: str = "http://localhost:11434"
    openai_base_url: str = ""  # OpenAI API 相容端點 (vLLM, LMStudio, etc.)

    def get_model(self) -> str:
        """獲取模型名稱，若未指定則使用預設"""
        if self.model:
            return self.model
        defaults = {
            "anthropic": "claude-3-5-haiku-20241022",
            "openai": "gpt-4o-mini",
            "gemini": "gemini-1.5-flash",
            "ollama": "llama3.2",
            "openai-compatible": "default",
        }
        return defaults.get(self.provider, "llama3.2")


@dataclass
class WorkSummary:
    """工作摘要"""
    description: str
    suggested_issue: Optional[str] = None
    category: Optional[str] = None


def get_llm_config() -> LLMConfig:
    """從環境變數獲取 LLM 配置

    環境變數:
        LLM_PROVIDER: anthropic, openai, gemini, ollama, openai-compatible
        LLM_MODEL: 模型名稱
        OLLAMA_HOST: Ollama 主機 (預設 http://localhost:11434)
        OPENAI_BASE_URL: OpenAI 相容 API 端點 (vLLM, LMStudio 等)
        ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_API_KEY: API keys
    """
    provider = os.environ.get("LLM_PROVIDER", "ollama").lower()
    model = os.environ.get("LLM_MODEL", "")
    ollama_host = os.environ.get("OLLAMA_HOST", "http://localhost:11434")
    openai_base_url = os.environ.get("OPENAI_BASE_URL", "")

    # 自動偵測 API key
    api_key = ""
    if provider == "anthropic":
        api_key = os.environ.get("ANTHROPIC_API_KEY", "")
    elif provider in ("openai", "openai-compatible"):
        api_key = os.environ.get("OPENAI_API_KEY", "")
    elif provider == "gemini":
        api_key = os.environ.get("GOOGLE_API_KEY", "") or os.environ.get("GEMINI_API_KEY", "")

    return LLMConfig(
        provider=provider,
        model=model,
        api_key=api_key,
        ollama_host=ollama_host,
        openai_base_url=openai_base_url
    )


def call_llm(prompt: str, config: LLMConfig = None) -> Optional[str]:
    """
    呼叫 LLM API

    Args:
        prompt: 提示詞
        config: LLM 配置，若為 None 則從環境變數讀取

    Returns:
        LLM 回應文字，失敗則返回 None
    """
    if config is None:
        config = get_llm_config()

    try:
        if config.provider == "anthropic":
            return _call_anthropic(prompt, config)
        elif config.provider == "openai":
            return _call_openai(prompt, config)
        elif config.provider == "openai-compatible":
            return _call_openai_compatible(prompt, config)
        elif config.provider == "gemini":
            return _call_gemini(prompt, config)
        elif config.provider == "ollama":
            return _call_ollama(prompt, config)
        else:
            print(f"Unknown LLM provider: {config.provider}")
            return None
    except Exception as e:
        print(f"LLM error ({config.provider}): {e}")
        return None


def _call_anthropic(prompt: str, config: LLMConfig) -> Optional[str]:
    """呼叫 Anthropic Claude API"""
    import anthropic

    if not config.api_key:
        return None

    client = anthropic.Anthropic(api_key=config.api_key)
    response = client.messages.create(
        model=config.get_model(),
        max_tokens=200,
        messages=[{"role": "user", "content": prompt}]
    )
    return response.content[0].text.strip()


def _call_openai(prompt: str, config: LLMConfig) -> Optional[str]:
    """呼叫 OpenAI API"""
    import openai

    if not config.api_key:
        return None

    client = openai.OpenAI(api_key=config.api_key)
    response = client.chat.completions.create(
        model=config.get_model(),
        max_tokens=200,
        messages=[{"role": "user", "content": prompt}]
    )
    return response.choices[0].message.content.strip()


def _call_openai_compatible(prompt: str, config: LLMConfig) -> Optional[str]:
    """呼叫 OpenAI API 相容端點 (vLLM, LMStudio, LocalAI, etc.)"""
    import requests

    base_url = config.openai_base_url.rstrip('/')
    if not base_url:
        print("OPENAI_BASE_URL not set for openai-compatible provider")
        return None

    url = f"{base_url}/v1/chat/completions"
    headers = {
        "Content-Type": "application/json",
    }
    if config.api_key:
        headers["Authorization"] = f"Bearer {config.api_key}"

    payload = {
        "model": config.get_model(),
        "max_tokens": 200,
        "messages": [{"role": "user", "content": prompt}]
    }

    response = requests.post(url, json=payload, headers=headers, timeout=30)
    response.raise_for_status()
    return response.json()["choices"][0]["message"]["content"].strip()


def _call_gemini(prompt: str, config: LLMConfig) -> Optional[str]:
    """呼叫 Google Gemini API"""
    import google.generativeai as genai

    if not config.api_key:
        return None

    genai.configure(api_key=config.api_key)
    model = genai.GenerativeModel(config.get_model())
    response = model.generate_content(prompt)
    return response.text.strip()


def _call_ollama(prompt: str, config: LLMConfig) -> Optional[str]:
    """呼叫 Ollama API (本地)"""
    import requests

    url = f"{config.ollama_host}/api/generate"
    payload = {
        "model": config.get_model(),
        "prompt": prompt,
        "stream": False,
        "options": {
            "num_predict": 200
        }
    }

    response = requests.post(url, json=payload, timeout=30)
    response.raise_for_status()
    return response.json().get("response", "").strip()


def summarize_work(
    project_name: str,
    todos: list[str],
    summaries: list[str],
    known_issues: dict[str, str] = None,
    max_length: int = 100,
    config: LLMConfig = None
) -> WorkSummary:
    """
    使用 LLM 彙整工作內容

    Args:
        project_name: 專案名稱
        todos: 完成的 todos
        summaries: 工作摘要
        known_issues: 已知的專案-Issue 對應
        max_length: 描述最大長度
        config: LLM 配置

    Returns:
        WorkSummary
    """
    if config is None:
        config = get_llm_config()

    # 構建 prompt
    prompt = _build_prompt(project_name, todos, summaries, known_issues, max_length)

    # 呼叫 LLM
    result = call_llm(prompt, config)

    if result:
        return _parse_llm_response(result)
    else:
        return _simple_summarize(project_name, todos, summaries, max_length)


def _build_prompt(
    project_name: str,
    todos: list[str],
    summaries: list[str],
    known_issues: dict[str, str],
    max_length: int
) -> str:
    """構建 LLM prompt"""
    todos_text = "\n".join(f"- {t}" for t in todos) if todos else "無"
    summaries_text = "\n".join(f"- {s[:80]}" for s in summaries[:3]) if summaries else "無"

    return f"""請幫我彙整以下工作內容，生成一個簡潔的 Jira worklog 描述。

專案: {project_name}

完成的任務:
{todos_text}

工作摘要:
{summaries_text}

請只回覆一行簡潔的工作描述（最多{max_length}字），使用繁體中文，不要有其他文字。
重點描述完成了什麼，適合作為 Jira worklog。"""


def _parse_llm_response(response: str) -> WorkSummary:
    """解析 LLM 回應"""
    description = response.strip()

    # 清理常見格式
    if "描述:" in description:
        description = description.split("描述:")[-1].strip()
    elif "描述：" in description:
        description = description.split("描述：")[-1].strip()

    description = description.strip('"').strip("'").strip()

    # 移除換行，只取第一行
    if "\n" in description:
        description = description.split("\n")[0].strip()

    return WorkSummary(description=description)


def _simple_summarize(
    project_name: str,
    todos: list[str],
    summaries: list[str],
    max_length: int
) -> WorkSummary:
    """簡單的彙整邏輯（不使用 LLM）"""
    if todos:
        if len(todos) == 1:
            description = f"完成: {todos[0]}"
        else:
            description = f"完成: {', '.join(todos[:3])}"
            if len(todos) > 3:
                description += f" 等 {len(todos)} 項"
    elif summaries:
        description = summaries[0]
    else:
        description = f"Work on {project_name}"

    if len(description) > max_length:
        description = description[:max_length-3] + "..."

    return WorkSummary(description=description)


def batch_summarize(
    entries: list[dict],
    known_issues: dict[str, str] = None,
    config: LLMConfig = None
) -> list[dict]:
    """
    批量彙整多個工作項目

    Args:
        entries: [{'project': ProjectSummary, 'entry': DailyProjectEntry}, ...]
        known_issues: 已知的專案-Issue 對應
        config: LLM 配置

    Returns:
        更新後的 entries
    """
    if config is None:
        config = get_llm_config()

    for e in entries:
        project = e['project']
        entry = e['entry']

        summary = summarize_work(
            project_name=project.project_name,
            todos=entry.todos,
            summaries=entry.summaries,
            known_issues=known_issues,
            config=config
        )

        e['description'] = summary.description

    return entries


def test_llm_connection(config: LLMConfig = None) -> tuple[bool, str]:
    """測試 LLM 連接"""
    if config is None:
        config = get_llm_config()

    try:
        result = call_llm("回覆 OK", config)
        if result:
            return True, f"Connected to {config.provider} ({config.get_model()})"
        return False, f"No response from {config.provider}"
    except Exception as e:
        return False, f"Connection failed: {e}"
