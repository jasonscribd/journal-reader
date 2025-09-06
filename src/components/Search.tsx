import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { 
  Search as SearchIcon, 
  Filter, 
  Calendar, 
  Tag, 
  FileText, 
  Clock,
  Zap,
  Brain,
  Layers,
  ChevronDown,
  ChevronUp,
  X
} from "lucide-react";

interface SearchResultItem {
  id: string;
  title?: string;
  body: string;
  entry_date: string;
  source_path: string;
  source_type: string;
  tags: string[];
  score: number;
  snippet: string;
  rank_source: string;
}

// Simplified for FTS demo

export function Search() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<any[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [searchTime, setSearchTime] = useState<number>(0);
  const [totalCount, setTotalCount] = useState(0);
  const [showFilters, setShowFilters] = useState(false);
  
  // Filter states
  const [dateRange, setDateRange] = useState<[string, string]>(["", ""]);
  
  // Available filter options (would come from backend in real app)
  const availableTags = ["personal", "work", "travel", "ideas", "goals", "reflection"];
  const availableSourceTypes = ["txt", "docx"];

  const executeSearch = useCallback(async () => {
    if (!query.trim()) {
      setResults([]);
      return;
    }

    setIsSearching(true);
    
    try {
      const start = performance.now();
      const response = await invoke<any[]>("search_entries_simple", { query: query.trim(), limit: 50 });
      setResults(response);
      setTotalCount(response.length);
      setSearchTime(Math.round(performance.now() - start));
    } catch (error) {
      console.error("Search failed:", error);
      setResults([]);
      setTotalCount(0);
    } finally {
      setIsSearching(false);
    }
  }, [query, dateRange]);

  // Trigger search explicitly to avoid frequent re-renders
  const onSubmit = async () => {
    await executeSearch();
  };

  const clearFilters = () => setDateRange(["", ""]);

  const removeTag = (_tag: string) => {};
  const removeSourceType = (_type: string) => {};

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  const formatFileSize = (path: string) => {
    // Mock file size - in real app would come from backend
    return "2.1 KB";
  };

  const getSearchTypeIcon = (type: string) => {
    switch (type) {
      case "fulltext":
        return <FileText className="w-4 h-4" />;
      case "semantic":
        return <Brain className="w-4 h-4" />;
      case "hybrid":
        return <Layers className="w-4 h-4" />;
      default:
        return <SearchIcon className="w-4 h-4" />;
    }
  };

  const getSearchTypeColor = (type: string) => {
    switch (type) {
      case "fulltext":
        return "bg-blue-100 text-blue-800";
      case "semantic":
        return "bg-purple-100 text-purple-800";
      case "hybrid":
        return "bg-green-100 text-green-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Search</h2>
        <p className="text-muted-foreground">Find entries with hybrid full-text and semantic search</p>
      </div>

      {/* Search Input */}
      <Card>
        <CardContent className="pt-6">
          <div className="space-y-4">
            <div className="flex gap-2">
              <div className="flex-1 relative">
                <SearchIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground w-4 h-4" />
                <Input
                  placeholder="Search your journal entries..."
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  className="pl-10"
                  onKeyDown={(e) => { if (e.key === 'Enter') onSubmit(); }}
                />
              </div>
              <Button onClick={onSubmit} disabled={!query.trim() || isSearching}>Search</Button>
              <Button
                variant="outline"
                onClick={() => setShowFilters(!showFilters)}
                className="flex items-center gap-2"
              >
                <Filter className="w-4 h-4" />
                Filters
                {showFilters ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
              </Button>
            </div>

            {/* Active Filters Display */}
            {(dateRange[0]) && (
              <div className="flex flex-wrap gap-2 items-center">
                <span className="text-sm text-muted-foreground">Active filters:</span>
                
                {dateRange[0] && (
                  <Badge variant="secondary" className="flex items-center gap-1">
                    <Calendar className="w-3 h-3" />
                    {formatDate(dateRange[0])} - {formatDate(dateRange[1])}
                    <X className="w-3 h-3 cursor-pointer" onClick={() => setDateRange(["", ""])} />
                  </Badge>
                )}
                
                <Button variant="ghost" size="sm" onClick={clearFilters}>
                  Clear all
                </Button>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Filters Panel */}
      {showFilters && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Filter className="w-5 h-5" />
              Search Filters
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* Date Range */}
              <div className="space-y-2">
                <Label>Date Range</Label>
                <div className="flex gap-2">
                  <Input
                    type="date"
                    value={dateRange[0]}
                    onChange={(e) => setDateRange([e.target.value, dateRange[1]])}
                    placeholder="Start date"
                  />
                  <Input
                    type="date"
                    value={dateRange[1]}
                    onChange={(e) => setDateRange([dateRange[0], e.target.value])}
                    placeholder="End date"
                  />
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Search Results */}
      {query && (
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <SearchIcon className="w-5 h-5" />
                Search Results
              </CardTitle>
              <div className="flex items-center gap-4 text-sm text-muted-foreground">
                {isSearching ? (
                  <div className="flex items-center gap-2">
                    <Clock className="w-4 h-4 animate-spin" />
                    Searching...
                  </div>
                ) : (
                  <>
                    <span>{totalCount} results</span>
                    <span>{searchTime}ms</span>
                  </>
                )}
              </div>
            </div>
          </CardHeader>
          <CardContent>
            {results.length === 0 && !isSearching ? (
              <div className="text-center py-8 text-muted-foreground">
                <SearchIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
                <p>No results found for "{query}"</p>
                <p className="text-sm">Try adjusting your search terms or filters</p>
              </div>
            ) : (
              <div className="space-y-4">
                {results.map((result: any, index) => (
                  <div key={result.id} className="border rounded-lg p-4 hover:bg-muted/50 transition-colors">
                    <div className="flex items-start justify-between mb-2">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <h3 className="font-medium">
                            {result.title || `Entry ${result.id.slice(0, 8)}`}
                          </h3>
                        </div>
                        <div className="flex items-center gap-4 text-sm text-muted-foreground mb-2">
                          <span className="flex items-center gap-1">
                            <Calendar className="w-3 h-3" />
                            {formatDate(result.entry_date)}
                          </span>
                          <span className="text-xs">ID: {result.id}</span>
                        </div>
                      </div>
                    </div>
                    
                    <p className="text-sm mb-3 leading-relaxed">
                      {result.preview}
                    </p>
                    
                    {result.tags.length > 0 && (
                      <div className="flex flex-wrap gap-1">
                        {result.tags.map(tag => (
                          <Badge key={tag} variant="secondary" size="sm">
                            {tag}
                          </Badge>
                        ))}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Empty State */}
      {!query && (
        <Card>
          <CardContent className="pt-6">
            <div className="text-center py-12">
              <SearchIcon className="w-16 h-16 mx-auto mb-4 text-muted-foreground opacity-50" />
              <h3 className="text-lg font-medium mb-2">Search Your Journal</h3>
              <p className="text-muted-foreground mb-4">
                Enter a search query to find entries using our hybrid search engine
              </p>
              <div className="flex justify-center gap-4 text-sm text-muted-foreground">
                <div className="flex items-center gap-2">
                  <FileText className="w-4 h-4" />
                  Full-text search
                </div>
                <div className="flex items-center gap-2">
                  <Brain className="w-4 h-4" />
                  Semantic search
                </div>
                <div className="flex items-center gap-2">
                  <Layers className="w-4 h-4" />
                  Hybrid ranking
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}




