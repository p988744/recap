import { FileText, RefreshCw, Award, Zap } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { TooltipProvider } from '@/components/ui/tooltip'
import { useAuth } from '@/lib/auth'
import { useReports, type ReportTab } from './hooks'
import { WorkReportTab, PEReportTab, TempoReportTab } from './components'

export function Reports() {
  const { token, isAuthenticated } = useAuth()
  const reportsState = useReports(isAuthenticated, token)

  if (reportsState.loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
      </div>
    )
  }

  return (
    <TooltipProvider>
      <div className="space-y-12">
        {/* Header */}
        <header className="flex items-start justify-between animate-fade-up opacity-0 delay-1">
          <div>
            <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
              工作報告
            </p>
            <h1 className="font-display text-4xl text-foreground tracking-tight">報告中心</h1>
          </div>
          <Button variant="ghost" onClick={reportsState.refresh}>
            <RefreshCw className="w-4 h-4 mr-2" strokeWidth={1.5} />
            重新整理
          </Button>
        </header>

        {/* Report Type Tabs */}
        <section className="animate-fade-up opacity-0 delay-2">
          <Tabs
            value={reportsState.activeTab}
            onValueChange={(v) => reportsState.setActiveTab(v as ReportTab)}
          >
            <TabsList>
              <TabsTrigger value="work" className="gap-2">
                <FileText className="w-4 h-4" strokeWidth={1.5} />
                工作報告
              </TabsTrigger>
              <TabsTrigger value="pe" className="gap-2">
                <Award className="w-4 h-4" strokeWidth={1.5} />
                績效考核
              </TabsTrigger>
              <TabsTrigger value="tempo" className="gap-2">
                <Zap className="w-4 h-4" strokeWidth={1.5} />
                Tempo 報告
              </TabsTrigger>
            </TabsList>

            {/* Work Report Tab */}
            <TabsContent value="work">
              <WorkReportTab
                data={reportsState.data}
                personalReport={reportsState.personalReport}
                period={reportsState.period}
                setPeriod={reportsState.setPeriod}
              />
            </TabsContent>

            {/* PE Report Tab */}
            <TabsContent value="pe">
              <PEReportTab
                peReport={reportsState.peReport}
                peYear={reportsState.peYear}
                setPEYear={reportsState.setPEYear}
                peHalf={reportsState.peHalf}
                setPEHalf={reportsState.setPEHalf}
              />
            </TabsContent>

            {/* Tempo Report Tab */}
            <TabsContent value="tempo">
              <TempoReportTab
                tempoReport={reportsState.tempoReport}
                tempoPeriod={reportsState.tempoPeriod}
                setTempoPeriod={reportsState.setTempoPeriod}
                tempoLoading={reportsState.tempoLoading}
              />
            </TabsContent>
          </Tabs>
        </section>
      </div>
    </TooltipProvider>
  )
}
