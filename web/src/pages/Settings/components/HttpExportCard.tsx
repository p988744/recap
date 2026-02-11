import {
  Globe,
  Plus,
  Save,
  Loader2,
  Trash2,
  CheckCircle2,
  XCircle,
  Eye,
  EyeOff,
  FlaskConical,
  Plug,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useHttpExportForm } from '../hooks/useHttpExportForm'

const AVAILABLE_FIELDS = [
  'title', 'description', 'hours', 'date', 'source',
  'jira_issue_key', 'project_name', 'category', 'llm_summary',
]

export function HttpExportCard() {
  const form = useHttpExportForm()

  if (form.loading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Loader2 className="w-4 h-4 animate-spin" />
          <span className="text-sm">Loading...</span>
        </div>
      </Card>
    )
  }

  return (
    <Card className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Globe className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <h3 className="font-medium text-foreground">HTTP Export</h3>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => form.selectConfig(null)}
        >
          <Plus className="w-3.5 h-3.5 mr-1" strokeWidth={1.5} />
          New Endpoint
        </Button>
      </div>

      {/* Endpoint list */}
      {form.configs.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {form.configs.map((c) => (
            <button
              key={c.id}
              onClick={() => form.selectConfig(c.id)}
              className={`px-3 py-1.5 text-xs rounded-md border transition-colors ${
                form.selectedId === c.id
                  ? 'border-foreground/40 bg-foreground/5 text-foreground'
                  : 'border-border text-muted-foreground hover:border-foreground/20'
              }`}
            >
              {c.name}
              {c.enabled && (
                <CheckCircle2 className="w-3 h-3 ml-1 inline-block text-green-500" />
              )}
            </button>
          ))}
        </div>
      )}

      {/* Form */}
      <div className="space-y-4">
        {/* Name */}
        <div className="space-y-1.5">
          <Label htmlFor="http-export-name" className="text-xs">Name</Label>
          <Input
            id="http-export-name"
            value={form.name}
            onChange={(e) => form.set({ name: e.target.value })}
            placeholder="My API"
          />
        </div>

        {/* URL + Method */}
        <div className="grid grid-cols-[1fr_120px] gap-3">
          <div className="space-y-1.5">
            <Label htmlFor="http-export-url" className="text-xs">URL</Label>
            <Input
              id="http-export-url"
              value={form.url}
              onChange={(e) => form.set({ url: e.target.value })}
              placeholder="https://api.example.com/logs"
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs">Method</Label>
            <Select
              value={form.method}
              onValueChange={(v) => form.set({ method: v })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="POST">POST</SelectItem>
                <SelectItem value="PUT">PUT</SelectItem>
                <SelectItem value="PATCH">PATCH</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* Auth */}
        <div className="space-y-1.5">
          <Label className="text-xs">Authentication</Label>
          <Select
            value={form.authType}
            onValueChange={(v) => form.set({ authType: v })}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">None</SelectItem>
              <SelectItem value="bearer">Bearer Token</SelectItem>
              <SelectItem value="basic">Basic Auth</SelectItem>
              <SelectItem value="header">Custom Header</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {form.authType === 'header' && (
          <div className="space-y-1.5">
            <Label htmlFor="http-export-header-name" className="text-xs">Header Name</Label>
            <Input
              id="http-export-header-name"
              value={form.authHeaderName}
              onChange={(e) => form.set({ authHeaderName: e.target.value })}
              placeholder="X-API-Key"
            />
          </div>
        )}

        {form.authType !== 'none' && (
          <div className="space-y-1.5">
            <Label htmlFor="http-export-token" className="text-xs">
              {form.authType === 'basic' ? 'Credentials (user:pass)' : 'Token'}
            </Label>
            <div className="relative">
              <Input
                id="http-export-token"
                type={form.showToken ? 'text' : 'password'}
                value={form.authToken}
                onChange={(e) => form.set({ authToken: e.target.value })}
                placeholder={form.selectedId ? '(unchanged if empty)' : 'Enter token'}
              />
              <button
                type="button"
                onClick={() => form.set({ showToken: !form.showToken })}
                className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              >
                {form.showToken ? (
                  <EyeOff className="w-4 h-4" />
                ) : (
                  <Eye className="w-4 h-4" />
                )}
              </button>
            </div>
          </div>
        )}

        {/* Payload Template */}
        <div className="space-y-1.5">
          <Label htmlFor="http-export-template" className="text-xs">Payload Template</Label>
          <Textarea
            id="http-export-template"
            value={form.payloadTemplate}
            onChange={(e) => form.set({ payloadTemplate: e.target.value })}
            rows={6}
            className="font-mono text-xs"
            placeholder='{"summary": "{{title}}", "hours": {{hours}}}'
          />
          <p className="text-[10px] text-muted-foreground">
            Available: {AVAILABLE_FIELDS.map((f) => `{{${f}}}`).join(', ')}
          </p>
        </div>

        {/* LLM Prompt */}
        <div className="space-y-1.5">
          <Label htmlFor="http-export-llm-prompt" className="text-xs">
            LLM Summary Prompt (optional)
          </Label>
          <Textarea
            id="http-export-llm-prompt"
            value={form.llmPrompt}
            onChange={(e) => form.set({ llmPrompt: e.target.value })}
            rows={2}
            className="font-mono text-xs"
            placeholder="Summarize: {{description}}"
          />
        </div>

        {/* Batch mode */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <Checkbox
              id="http-export-batch"
              checked={form.batchMode}
              onCheckedChange={(v) => form.set({ batchMode: !!v })}
            />
            <Label htmlFor="http-export-batch" className="text-xs font-normal cursor-pointer">
              Batch mode (single POST with array)
            </Label>
          </div>
          {form.batchMode && (
            <Input
              value={form.batchWrapperKey}
              onChange={(e) => form.set({ batchWrapperKey: e.target.value })}
              placeholder="items"
              className="w-28 h-7 text-xs"
            />
          )}
        </div>

        {/* Validate result */}
        {form.validateResult && (
          <div
            className={`p-3 rounded-md border text-xs font-mono ${
              form.validateResult.valid
                ? 'border-green-500/30 bg-green-500/5'
                : 'border-red-500/30 bg-red-500/5'
            }`}
          >
            {form.validateResult.valid ? (
              <>
                <p className="text-green-600 mb-1">
                  Valid template. Fields: {form.validateResult.fields_used.join(', ')}
                </p>
                {form.validateResult.sample_output && (
                  <pre className="text-muted-foreground whitespace-pre-wrap break-all">
                    {JSON.stringify(JSON.parse(form.validateResult.sample_output), null, 2)}
                  </pre>
                )}
              </>
            ) : (
              <p className="text-red-600">{form.validateResult.error}</p>
            )}
          </div>
        )}

        {/* Test result */}
        {form.testResult && (
          <div
            className={`p-3 rounded-md border text-xs ${
              form.testResult.success
                ? 'border-green-500/30 bg-green-500/5'
                : 'border-red-500/30 bg-red-500/5'
            }`}
          >
            <div className="flex items-center gap-1.5">
              {form.testResult.success ? (
                <CheckCircle2 className="w-3.5 h-3.5 text-green-500" />
              ) : (
                <XCircle className="w-3.5 h-3.5 text-red-500" />
              )}
              <span>{form.testResult.message}</span>
            </div>
          </div>
        )}

        {/* Message */}
        {form.message && (
          <p
            className={`text-xs ${
              form.message.type === 'success' ? 'text-green-600' : 'text-red-600'
            }`}
          >
            {form.message.text}
          </p>
        )}

        {/* Action buttons */}
        <div className="flex items-center justify-between pt-2">
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={form.validateTemplate}
              disabled={form.validating}
            >
              {form.validating ? (
                <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />
              ) : (
                <FlaskConical className="w-3.5 h-3.5 mr-1" strokeWidth={1.5} />
              )}
              Validate
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={form.testConnection}
              disabled={form.testing || !form.selectedId}
            >
              {form.testing ? (
                <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />
              ) : (
                <Plug className="w-3.5 h-3.5 mr-1" strokeWidth={1.5} />
              )}
              Test
            </Button>
          </div>

          <div className="flex items-center gap-2">
            {form.selectedId && (
              <Button
                variant="ghost"
                size="sm"
                onClick={form.deleteConfig}
                disabled={form.deleting}
                className="text-red-500 hover:text-red-600 hover:bg-red-50"
              >
                {form.deleting ? (
                  <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />
                ) : (
                  <Trash2 className="w-3.5 h-3.5 mr-1" strokeWidth={1.5} />
                )}
                Delete
              </Button>
            )}
            <Button
              size="sm"
              onClick={form.saveConfig}
              disabled={form.saving || !form.name || !form.url}
            >
              {form.saving ? (
                <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />
              ) : (
                <Save className="w-3.5 h-3.5 mr-1" strokeWidth={1.5} />
              )}
              Save
            </Button>
          </div>
        </div>
      </div>
    </Card>
  )
}
