import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeRaw from 'rehype-raw'

interface MarkdownSummaryProps {
  content: string
  className?: string
}

export function MarkdownSummary({ content, className = '' }: MarkdownSummaryProps) {
  return (
    <div className={`prose prose-sm max-w-none dark:prose-invert break-words
      prose-p:my-1 prose-p:leading-relaxed prose-p:text-muted-foreground
      prose-ul:my-1.5 prose-ul:pl-4
      prose-li:my-0 prose-li:text-muted-foreground prose-li:leading-relaxed
      prose-strong:text-foreground prose-strong:font-medium
      prose-code:text-xs prose-code:bg-muted/50 prose-code:px-1 prose-code:py-0.5 prose-code:rounded prose-code:font-mono prose-code:before:content-none prose-code:after:content-none prose-code:break-all
      prose-table:my-2 prose-table:text-sm
      prose-th:bg-muted/50 prose-th:px-3 prose-th:py-2 prose-th:text-left prose-th:font-medium prose-th:text-foreground prose-th:border prose-th:border-border
      prose-td:px-3 prose-td:py-2 prose-td:text-muted-foreground prose-td:border prose-td:border-border
      prose-tr:border-b prose-tr:border-border
      text-sm text-muted-foreground ${className}`}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeRaw]}>{content}</ReactMarkdown>
    </div>
  )
}
