import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { BookOpen, Settings as SettingsIcon, Calendar, FileText, Search as SearchIcon } from "lucide-react";
import "./App.css";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Settings } from "@/components/Settings";
import { Import } from "@/components/Import";
import { Timeline } from "@/components/Timeline";
import { Search } from "@/components/Search";

function App() {
  const [isInitialized, setIsInitialized] = useState(false);
  const [currentView, setCurrentView] = useState<'timeline' | 'search' | 'import' | 'settings'>('timeline');

  useEffect(() => {
    // Initialize the database on startup
    const initApp = async () => {
      try {
        await invoke("init_database");
        setIsInitialized(true);
      } catch (error) {
        console.error("Failed to initialize app:", error);
      }
    };

    initApp();
  }, []);

  if (!isInitialized) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-center">
          <BookOpen className="w-12 h-12 mx-auto mb-4 animate-pulse" />
          <p className="text-lg">Initializing Journal Reader...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <div className="w-64 border-r bg-card">
        <div className="p-6">
          <div className="flex items-center gap-2 mb-8">
            <BookOpen className="w-8 h-8 text-primary" />
            <h1 className="text-xl font-bold">Journal Reader</h1>
          </div>
          
          <nav className="space-y-2">
            <Button
              variant={currentView === 'timeline' ? 'default' : 'ghost'}
              className="w-full justify-start"
              onClick={() => setCurrentView('timeline')}
            >
              <Calendar className="w-4 h-4 mr-2" />
              Timeline
            </Button>
            <Button
              variant={currentView === 'search' ? 'default' : 'ghost'}
              className="w-full justify-start"
              onClick={() => setCurrentView('search')}
            >
              <SearchIcon className="w-4 h-4 mr-2" />
              Search
            </Button>
            <Button
              variant={currentView === 'import' ? 'default' : 'ghost'}
              className="w-full justify-start"
              onClick={() => setCurrentView('import')}
            >
              <FileText className="w-4 h-4 mr-2" />
              Import
            </Button>
            <Button
              variant={currentView === 'settings' ? 'default' : 'ghost'}
              className="w-full justify-start"
              onClick={() => setCurrentView('settings')}
            >
              <SettingsIcon className="w-4 h-4 mr-2" />
              Settings
            </Button>
          </nav>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col">
        {(
          <header className="border-b bg-card px-6 py-4">
            <h2 className="text-2xl font-semibold capitalize">{currentView}</h2>
          </header>
        )}
        
        <main className="flex-1 overflow-auto">
          {currentView === 'timeline' && <div className="p-6"><TimelineView /></div>}
          {currentView === 'search' && <div className="p-6"><SearchView /></div>}
          {currentView === 'import' && <div className="p-6"><ImportView /></div>}
          {currentView === 'settings' && <div className="p-6"><SettingsView /></div>}
        </main>
      </div>
    </div>
  );
}

function TimelineView() {
  return <Timeline />;
}

function SearchView() {
  return <Search />;
}

function ImportView() {
  return <Import />;
}

function SettingsView() {
  return <Settings />;
}

export default App;
