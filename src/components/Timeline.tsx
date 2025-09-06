import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import * as Dialog from "@radix-ui/react-dialog";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { 
  Calendar, 
  ChevronLeft,
  ChevronRight,
  Clock,
  FileText,
  Tag,
} from "lucide-react";

interface EntryPreview {
  id: string;
  title?: string;
  preview: string;
  entry_date: string;
  tags: string[];
}

interface MonthCount { month: number; count: number; }

export function Timeline() {
  const [years, setYears] = useState<number[]>([]);
  const [monthCounts, setMonthCounts] = useState<MonthCount[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedYear, setSelectedYear] = useState<number>(new Date().getFullYear());
  const [selectedMonth, setSelectedMonth] = useState<number | null>(null);
  const [entries, setEntries] = useState<EntryPreview[]>([]);
  const [dbInfo, setDbInfo] = useState<{ db_path: string; total_entries: number; years: number[] } | null>(null);
  const [isEntryOpen, setIsEntryOpen] = useState(false);
  const [entryLoading, setEntryLoading] = useState(false);
  const [selectedEntry, setSelectedEntry] = useState<EntryPreview | null>(null);

  useEffect(() => {
    loadYears();
    // quick diagnostics for user
    invoke<any>("get_db_diagnostics").then((info) => setDbInfo(info)).catch(() => {});
  }, []);

  useEffect(() => {
    if (selectedYear) {
      loadMonthCounts(selectedYear);
      setSelectedMonth(null);
      setEntries([]);
    }
  }, [selectedYear]);

  useEffect(() => {
    if (selectedMonth) {
      loadEntries(selectedYear, selectedMonth);
    }
  }, [selectedMonth]);

  const loadYears = async () => {
    try {
      setIsLoading(true);
      const y = await invoke<number[]>("get_available_years");
      setYears(y.length > 0 ? y : [new Date().getFullYear()]);
      setSelectedYear(y[0] || new Date().getFullYear());
    } catch (error) {
      console.error("Failed to load timeline data:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const loadMonthCounts = async (year: number) => {
    try {
      const counts = await invoke<MonthCount[]>("get_month_counts_for_year", { year });
      setMonthCounts(counts);
    } catch (error) {
      console.error("Failed to load heatmap data:", error);
    }
  };

  const loadEntries = async (year: number, month: number) => {
    try {
      const list = await invoke<EntryPreview[]>("list_entries_for_month", { year, month });
      setEntries(list);
    } catch (error) {
      console.error("Failed to load day view:", error);
    }
  };

  const openEntry = async (entryId: string) => {
    try {
      setEntryLoading(true);
      const data = await invoke<EntryPreview | null>("get_entry_by_id", { id: entryId });
      if (data) {
        setSelectedEntry(data);
        setIsEntryOpen(true);
      }
    } catch (error) {
      console.error("Failed to load entry:", error);
    } finally {
      setEntryLoading(false);
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric'
    });
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <Clock className="w-8 h-8 mx-auto mb-2 animate-pulse" />
          <p>Loading timeline...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">Timeline</h2>
          <p className="text-muted-foreground">Browse by year → month. Click a month to view entries.</p>
        </div>
        <div className="flex items-center gap-2" />
      </div>
      {dbInfo && (
        <div className="text-xs text-muted-foreground">
          <div className="flex items-center justify-between">
            <span>DB: {dbInfo.db_path}</span>
            <span>Total entries: {dbInfo.total_entries}</span>
          </div>
          <Separator className="my-2" />
        </div>
      )}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Calendar className="w-5 h-5" />
              Year {selectedYear}
            </div>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={() => setSelectedYear(selectedYear - 1)}>
                <ChevronLeft className="w-4 h-4" />
              </Button>
              <span className="font-medium">{selectedYear}</span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setSelectedYear(selectedYear + 1)}
                disabled={selectedYear >= new Date().getFullYear()}
              >
                <ChevronRight className="w-4 h-4" />
              </Button>
            </div>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {/* 12-month grid with two colors */}
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
            {Array.from({ length: 12 }, (_, i) => i + 1).map((m) => {
              const count = monthCounts.find(mc => mc.month === m)?.count || 0;
              const hasEntries = count > 0;
              const label = new Date(2000, m - 1, 1).toLocaleString('en-US', { month: 'long' });
              return (
                <Button
                  key={m}
                  variant={hasEntries ? 'default' : 'outline'}
                  className="h-16 flex-col"
                  onClick={() => setSelectedMonth(m)}
                >
                  <span className="font-medium">{label}</span>
                  <span className="text-xs opacity-80">{count} {count === 1 ? 'entry' : 'entries'}</span>
                </Button>
              );
            })}
          </div>

          {/* Entries list */}
          {selectedMonth && (
            <div className="mt-6 space-y-3">
              <h3 className="font-medium">
                {new Date(selectedYear, selectedMonth - 1, 1).toLocaleString('en-US', { month: 'long', year: 'numeric' })}
              </h3>
              {entries.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <FileText className="w-12 h-12 mx-auto mb-4 opacity-50" />
                  <p>No entries for this month</p>
                </div>
              ) : (
                entries.map((entry) => (
                  <div key={entry.id} className="border rounded-lg p-4 cursor-pointer hover:bg-muted/50" onClick={() => openEntry(entry.id)}>
                    <div className="flex items-start justify-between mb-2">
                      <h3 className="font-medium">
                        {entry.title || `Entry ${entry.id.slice(0, 8)}`}
                      </h3>
                      <Badge variant="outline" size="sm">
                        {formatDate(entry.entry_date)}
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground mb-3 leading-relaxed">
                      {entry.preview}
                    </p>
                    {entry.tags.length > 0 && (
                      <div className="flex flex-wrap gap-1">
                        {entry.tags.map(tag => (
                          <Badge key={tag} variant="secondary" size="sm">
                            <Tag className="w-3 h-3 mr-1" />
                            {tag}
                          </Badge>
                        ))}
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Entry Modal */}
      <Dialog.Root open={isEntryOpen} onOpenChange={setIsEntryOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 bg-black/40" />
          <Dialog.Content className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-card border rounded-lg shadow-lg w-[90vw] max-w-2xl max-h-[80vh] overflow-auto p-6">
            <Dialog.Title className="text-xl font-semibold mb-2">
              {selectedEntry?.title || (selectedEntry ? `Entry ${selectedEntry.id.slice(0,8)}` : 'Entry')}
            </Dialog.Title>
            <div className="text-sm text-muted-foreground mb-4">
              {selectedEntry && formatDate(selectedEntry.entry_date)}
            </div>
            <div className="whitespace-pre-wrap leading-relaxed text-sm">
              {entryLoading ? 'Loading…' : (selectedEntry?.preview || '')}
            </div>
            <div className="mt-4 flex justify-end">
              <Button variant="outline" onClick={() => setIsEntryOpen(false)}>Close</Button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </div>
  );
}




