import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { getVersion } from '@tauri-apps/api/app'
import { useAuth } from '@/lib/auth'
import { Card, CardContent } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'

export function LoginPage() {
  const navigate = useNavigate()
  const { login, register } = useAuth()
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [version, setVersion] = useState('')

  useEffect(() => {
    getVersion().then(setVersion)
  }, [])

  // Login form
  const [loginUsername, setLoginUsername] = useState('')
  const [loginPassword, setLoginPassword] = useState('')

  // Register form
  const [regUsername, setRegUsername] = useState('')
  const [regPassword, setRegPassword] = useState('')
  const [regName, setRegName] = useState('')
  const [regEmail, setRegEmail] = useState('')
  const [regTitle, setRegTitle] = useState('')

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)

    // Validation
    if (!loginUsername.trim()) {
      setError('請輸入帳號')
      return
    }
    if (!loginPassword) {
      setError('請輸入密碼')
      return
    }

    setIsLoading(true)

    try {
      await login(loginUsername, loginPassword)
      navigate('/')
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Login failed'
      // Translate common error messages
      if (message.includes('Invalid credentials') || message.includes('invalid')) {
        setError('帳號或密碼錯誤')
      } else if (message.includes('not found') || message.includes('User not found')) {
        setError('此帳號不存在')
      } else {
        setError(message)
      }
    } finally {
      setIsLoading(false)
    }
  }

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)

    // Validation
    if (!regUsername.trim()) {
      setError('請輸入帳號')
      return
    }
    if (!regName.trim()) {
      setError('請輸入姓名')
      return
    }
    if (!regPassword) {
      setError('請輸入密碼')
      return
    }
    if (regPassword.length < 4) {
      setError('密碼至少需要 4 個字元')
      return
    }

    setIsLoading(true)

    try {
      await register(regUsername, regPassword, regName, regEmail.trim() || undefined, regTitle || undefined)
      navigate('/')
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Registration failed'
      // Translate common error messages
      if (message.includes('already exists') || message.includes('duplicate')) {
        setError('此帳號已被註冊')
      } else {
        setError(message)
      }
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-cream flex items-center justify-center p-4">
      <div className="w-full max-w-sm animate-fade-up opacity-0 delay-1">
        {/* Header */}
        <div className="text-center mb-10">
          <h1 className="font-display text-3xl text-foreground tracking-tight">Recap</h1>
          <p className="text-sm text-muted-foreground mt-2">
            自動回顧你的工作
          </p>
        </div>

        <Card className="border-border">
          <CardContent className="pt-6">
            {error && (
              <div className="mb-6 p-3 border-l-2 border-l-terracotta bg-terracotta/5 text-terracotta text-sm">
                {error}
              </div>
            )}

            <Tabs defaultValue="login" className="w-full">
              <TabsList className="grid w-full grid-cols-2 mb-6 bg-cream-200 p-1">
                <TabsTrigger
                  value="login"
                  className="text-sm data-[state=active]:bg-white data-[state=active]:text-foreground data-[state=inactive]:text-muted-foreground transition-all"
                >
                  登入
                </TabsTrigger>
                <TabsTrigger
                  value="register"
                  className="text-sm data-[state=active]:bg-white data-[state=active]:text-foreground data-[state=inactive]:text-muted-foreground transition-all"
                >
                  註冊
                </TabsTrigger>
              </TabsList>

              <TabsContent value="login">
                <form onSubmit={handleLogin} className="space-y-5" noValidate>
                  <div className="space-y-2">
                    <Label htmlFor="login-username">帳號</Label>
                    <Input
                      id="login-username"
                      type="text"
                      placeholder="your_account"
                      value={loginUsername}
                      onChange={(e) => setLoginUsername(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="login-password">密碼</Label>
                    <Input
                      id="login-password"
                      type="password"
                      value={loginPassword}
                      onChange={(e) => setLoginPassword(e.target.value)}
                    />
                  </div>
                  <Button
                    type="submit"
                    className="w-full"
                    disabled={isLoading}
                  >
                    {isLoading ? '登入中...' : '登入'}
                  </Button>
                </form>
              </TabsContent>

              <TabsContent value="register">
                <form onSubmit={handleRegister} className="space-y-5" noValidate>
                  <div className="space-y-2">
                    <Label htmlFor="reg-username">帳號</Label>
                    <Input
                      id="reg-username"
                      type="text"
                      placeholder="your_account"
                      value={regUsername}
                      onChange={(e) => setRegUsername(e.target.value)}
                    />
                    <p className="text-xs text-muted-foreground">用於登入</p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="reg-name">姓名</Label>
                    <Input
                      id="reg-name"
                      type="text"
                      placeholder="王小明"
                      value={regName}
                      onChange={(e) => setRegName(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="reg-password">密碼</Label>
                    <Input
                      id="reg-password"
                      type="password"
                      value={regPassword}
                      onChange={(e) => setRegPassword(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="reg-email">
                      Email <span className="text-muted-foreground text-xs">(選填)</span>
                    </Label>
                    <Input
                      id="reg-email"
                      type="text"
                      placeholder="you@company.com"
                      value={regEmail}
                      onChange={(e) => setRegEmail(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="reg-title">
                      職稱 <span className="text-muted-foreground text-xs">(選填)</span>
                    </Label>
                    <Input
                      id="reg-title"
                      type="text"
                      placeholder="Software Engineer"
                      value={regTitle}
                      onChange={(e) => setRegTitle(e.target.value)}
                    />
                  </div>
                  <Button
                    type="submit"
                    className="w-full"
                    disabled={isLoading}
                  >
                    {isLoading ? '建立帳號中...' : '建立帳號'}
                  </Button>
                </form>
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>

        <p className="text-center text-[10px] text-muted-foreground mt-6">
          {version ? `Recap v${version}` : 'Recap'}
        </p>
      </div>
    </div>
  )
}
