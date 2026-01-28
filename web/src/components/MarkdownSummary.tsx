import ReactMarkdown from 'react-markdown'

interface MarkdownSummaryProps {
  content: string
  className?: string
}

export function MarkdownSummary({ content, className = '' }: MarkdownSummaryProps) {
  return (
    <div className={`prose prose-sm max-w-none dark:prose-invert
      prose-p:my-1 prose-p:leading-relaxed prose-p:text-muted-foreground
      prose-ul:my-1.5 prose-ul:pl-4
      prose-li:my-0 prose-li:text-muted-foreground prose-li:leading-relaxed
      prose-strong:text-foreground prose-strong:font-medium
      prose-code:text-xs prose-code:bg-muted/50 prose-code:px-1 prose-code:py-0.5 prose-code:rounded prose-code:font-mono prose-code:before:content-none prose-code:after:content-none
      text-sm text-muted-foreground ${className}`}
    >
      <ReactMarkdown>{content}</ReactMarkdown>
    </div>
  )
}
