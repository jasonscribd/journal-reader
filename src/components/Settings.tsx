import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { 
  Settings as SettingsIcon,
  Database,
  Brain,
  FileText,
  Globe,
  Zap,
  Save,
  AlertCircle,
  CheckCircle,
  RefreshCw
} from "lucide-react";

interface Setting {
  key: string;
  value: string;
}

export function Settings() {
  const [settings, setSettings] = useState<Setting[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error', text: string } | null>(null);
  
  // Local state for settings
  const [aiProvider, setAiProvider] = useState("ollama");
  const [ollamaUrl, setOllamaUrl] = useState("http://localhost:11434");
  const [openaiApiKey, setOpenaiApiKey] = useState("");
  const [defaultModel, setDefaultModel] = useState("llama3.1:8b");
  const [embeddingModel, setEmbeddingModel] = useState("nomic-embed-text");
  const [maxContextEntries, setMaxContextEntries] = useState(5);
  const [searchResultsLimit, setSearchResultsLimit] = useState(20);
  const [autoTagging, setAutoTagging] = useState(true);
  const [googleClientId, setGoogleClientId] = useState("");
  const [googleConnected, setGoogleConnected] = useState(false);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const settingsData = await invoke<Setting[]>("get_settings");
      setSettings(settingsData);
      
      // Apply settings to local state
      settingsData.forEach(setting => {
        switch (setting.key) {
          case "ai_provider":
            setAiProvider(setting.value);
            break;
          case "ollama_url":
            setOllamaUrl(setting.value);
            break;
          case "openai_api_key":
            setOpenaiApiKey(setting.value);
            break;
          case "default_model":
            setDefaultModel(setting.value);
            break;
          case "embedding_model":
            setEmbeddingModel(setting.value);
            break;
          case "max_context_entries":
            setMaxContextEntries(parseInt(setting.value) || 5);
            break;
          case "search_results_limit":
            setSearchResultsLimit(parseInt(setting.value) || 20);
            break;
          case "auto_tagging":
            setAutoTagging(setting.value === "true");
            break;
          case "google_client_id":
            setGoogleClientId(setting.value);
            break;
        }
      });
      try {
        const status = await invoke<{ connected: boolean }>("get_google_oauth_status");
        setGoogleConnected(status.connected);
      } catch {}
    } catch (error) {
      console.error("Failed to load settings:", error);
      setMessage({ type: 'error', text: 'Failed to load settings' });
    } finally {
      setLoading(false);
    }
  };

  const saveSettings = async () => {
    setSaving(true);
    setMessage(null);
    
    const settingsToUpdate = [
      { key: "ai_provider", value: aiProvider },
      { key: "ollama_url", value: ollamaUrl },
      { key: "openai_api_key", value: openaiApiKey },
      { key: "default_model", value: defaultModel },
      { key: "embedding_model", value: embeddingModel },
      { key: "max_context_entries", value: maxContextEntries.toString() },
      { key: "search_results_limit", value: searchResultsLimit.toString() },
      { key: "auto_tagging", value: autoTagging.toString() },
      { key: "google_client_id", value: googleClientId },
    ];

    try {
      for (const setting of settingsToUpdate) {
        await invoke("update_setting", {
          key: setting.key,
          value: setting.value,
        });
      }
      
      setMessage({ type: 'success', text: 'Settings saved successfully!' });
      await loadSettings(); // Reload to confirm
    } catch (error) {
      console.error("Failed to save settings:", error);
      setMessage({ type: 'error', text: 'Failed to save settings' });
    } finally {
      setSaving(false);
    }
  };

  const testConnection = async () => {
    try {
      setMessage(null);
      const ok = await invoke<boolean>("test_ai_connection");
      setMessage(ok ? { type: 'success', text: 'Ollama reachable' } : { type: 'error', text: 'Ollama not reachable' });
    } catch (error) {
      setMessage({ type: 'error', text: 'Connection test failed' });
    }
  };

  const connectGoogle = async () => {
    try {
      setMessage(null);
      const init = await invoke<{ auth_url: string, state: string, code_verifier: string }>("google_oauth_start");
      window.open(init.auth_url, "_blank");
      const code = prompt("Authorize in your browser, then paste the 'code' parameter from the redirected URL:") || "";
      if (!code) return;
      const ok = await invoke<boolean>("google_oauth_complete", { req: { code, state: init.state, codeVerifier: init.code_verifier } });
      setGoogleConnected(ok);
      setMessage(ok ? { type: 'success', text: 'Google Drive connected!' } : { type: 'error', text: 'Failed to connect Google Drive' });
    } catch (error) {
      setMessage({ type: 'error', text: 'Google auth failed' });
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <RefreshCw className="w-6 h-6 animate-spin mr-2" />
        <span>Loading settings...</span>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Settings</h2>
        <p className="text-muted-foreground">Configure your Journal Reader preferences</p>
      </div>

      {message && (
        <div className={`p-4 rounded-lg flex items-center gap-2 ${
          message.type === 'success' 
            ? 'bg-green-50 text-green-800 border border-green-200' 
            : 'bg-red-50 text-red-800 border border-red-200'
        }`}>
          {message.type === 'success' ? (
            <CheckCircle className="w-4 h-4" />
          ) : (
            <AlertCircle className="w-4 h-4" />
          )}
          {message.text}
        </div>
      )}

      {/* AI Provider Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Brain className="w-5 h-5" />
            AI Provider Configuration
          </CardTitle>
          <CardDescription>
            Configure your AI provider for semantic search and chat functionality
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="ai-provider">AI Provider</Label>
              <Select value={aiProvider} onValueChange={setAiProvider}>
                <SelectTrigger>
                  <SelectValue placeholder="Select AI provider" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="ollama">
                    <div className="flex items-center gap-2">
                      <Globe className="w-4 h-4" />
                      Ollama (Local)
                    </div>
                  </SelectItem>
                  <SelectItem value="openai">
                    <div className="flex items-center gap-2">
                      <Zap className="w-4 h-4" />
                      OpenAI
                    </div>
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="default-model">Default Model</Label>
              <Input
                id="default-model"
                value={defaultModel}
                onChange={(e) => setDefaultModel(e.target.value)}
                placeholder="llama3.1:8b"
              />
            </div>

            {aiProvider === "ollama" && (
              <div className="space-y-2">
                <Label htmlFor="ollama-url">Ollama URL</Label>
                <Input
                  id="ollama-url"
                  value={ollamaUrl}
                  onChange={(e) => setOllamaUrl(e.target.value)}
                  placeholder="http://localhost:11434"
                />
              </div>
            )}

            {aiProvider === "openai" && (
              <div className="space-y-2">
                <Label htmlFor="openai-key">OpenAI API Key</Label>
                <Input
                  id="openai-key"
                  type="password"
                  value={openaiApiKey}
                  onChange={(e) => setOpenaiApiKey(e.target.value)}
                  placeholder="sk-..."
                />
              </div>
            )}

            <div className="space-y-2">
              <Label htmlFor="embedding-model">Embedding Model</Label>
              <Input
                id="embedding-model"
                value={embeddingModel}
                onChange={(e) => setEmbeddingModel(e.target.value)}
                placeholder="nomic-embed-text"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="google-client-id">Google Client ID</Label>
              <Input
                id="google-client-id"
                value={googleClientId}
                onChange={(e) => setGoogleClientId(e.target.value)}
                placeholder="your-client-id.apps.googleusercontent.com"
              />
              <div className="flex items-center gap-2">
                <Button onClick={connectGoogle} size="sm" variant="outline">{googleConnected ? 'Reconnect Google Drive' : 'Connect Google Drive'}</Button>
                {googleConnected && <span className="text-xs text-green-700">Connected</span>}
              </div>
            </div>
          </div>

          <div className="flex gap-2">
            <Button onClick={testConnection} variant="outline" size="sm">
              Test Connection
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Search Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileText className="w-5 h-5" />
            Search Configuration
          </CardTitle>
          <CardDescription>
            Customize search behavior and performance
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="max-context">Max Context Entries</Label>
              <Input
                id="max-context"
                type="number"
                min="1"
                max="20"
                value={maxContextEntries}
                onChange={(e) => setMaxContextEntries(parseInt(e.target.value) || 5)}
              />
              <p className="text-sm text-muted-foreground">
                Maximum number of entries to use as context for AI responses
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="search-limit">Search Results Limit</Label>
              <Input
                id="search-limit"
                type="number"
                min="5"
                max="100"
                value={searchResultsLimit}
                onChange={(e) => setSearchResultsLimit(parseInt(e.target.value) || 20)}
              />
              <p className="text-sm text-muted-foreground">
                Maximum number of search results to display
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Database Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Database className="w-5 h-5" />
            Database & Import
          </CardTitle>
          <CardDescription>
            Database and import behavior settings
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label>Auto-tagging</Label>
              <p className="text-sm text-muted-foreground">
                Automatically suggest tags for imported entries using AI
              </p>
            </div>
            <Button
              variant={autoTagging ? "default" : "outline"}
              size="sm"
              onClick={() => setAutoTagging(!autoTagging)}
            >
              {autoTagging ? "Enabled" : "Disabled"}
            </Button>
          </div>

          <Separator />

          <div className="flex gap-2">
            <Button onClick={() => invoke("init_database")} variant="outline" size="sm">
              <Database className="w-4 h-4 mr-2" />
              Reinitialize Database
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Save Button */}
      <div className="flex justify-end gap-2">
        <Button onClick={loadSettings} variant="outline" disabled={saving}>
          <RefreshCw className="w-4 h-4 mr-2" />
          Reset
        </Button>
        <Button onClick={saveSettings} disabled={saving}>
          {saving ? (
            <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
          ) : (
            <Save className="w-4 h-4 mr-2" />
          )}
          Save Settings
        </Button>
      </div>
    </div>
  );
}
