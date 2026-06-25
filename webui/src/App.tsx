import { useState } from "react";
import { CategorySidebar } from "./components/CategorySidebar";
import { ProviderDetail } from "./components/ProviderDetail";
import { ThemeToggle } from "./components/ThemeToggle";
import { SettingsModal } from "./components/SettingsModal";
import { Button } from "./components/ui/button";

export default function App() {
  const [selectedProviderId, setSelectedProviderId] = useState<number | null>(null);
  const [selectedCategoryId, setSelectedCategoryId] = useState<number | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      {/* Titlebar */}
      <header className="h-10 flex items-center justify-between px-4 border-b border-border bg-card">
        <div className="flex items-center gap-2">
          <span className="text-lg font-bold">🔑</span>
          <span className="font-semibold text-sm">KeyPilot</span>
        </div>
        <div className="flex items-center gap-2">
          <ThemeToggle />
          <Button size="icon" variant="ghost" onClick={() => setSettingsOpen(true)} className="h-8 w-8">
            <span className="text-sm">⚙️</span>
          </Button>
        </div>
      </header>

      {/* Main content */}
      <div className="flex flex-1 overflow-hidden">
        <aside className="w-[300px] border-r border-border overflow-y-auto bg-card">
          <CategorySidebar
            selectedCategoryId={selectedCategoryId}
            onSelectCategory={setSelectedCategoryId}
            selectedProviderId={selectedProviderId}
            onSelectProvider={setSelectedProviderId}
          />
        </aside>
        <main className="flex-1 overflow-y-auto">
          <ProviderDetail providerId={selectedProviderId} />
        </main>
      </div>

      {/* Settings Modal */}
      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}
