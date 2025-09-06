import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import { 
  Tag, 
  Brain, 
  Wand2, 
  Check, 
  X, 
  Plus, 
  Settings, 
  BarChart3,
  Lightbulb,
  Zap,
  Clock,
  Target,
  AlertCircle,
  Trash2
} from "lucide-react";

interface VocabularyTag {
  name: string;
  description: string;
  aliases: string[];
  category: string;
  examples: string[];
}

interface ControlledVocabulary {
  tags: VocabularyTag[];
  aliases: Record<string, string>;
}

interface TagSuggestion {
  tag: string;
  confidence: number;
  reasoning: string;
  text_spans: string[];
}

interface TagExtractionResult {
  suggestions: TagSuggestion[];
  processing_time_ms: number;
  model_used: string;
}

interface TagStatistic {
  tag: string;
  count: number;
  percentage: number;
  recent_usage: string;
}

interface BulkTagResult {
  entry_id: string;
  success: boolean;
  suggestions?: TagSuggestion[];
  error?: string;
}

export function Tagging() {
  const [vocabulary, setVocabulary] = useState<ControlledVocabulary | null>(null);
  const [statistics, setStatistics] = useState<TagStatistic[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [currentTab, setCurrentTab] = useState<'extract' | 'vocabulary' | 'statistics' | 'bulk'>('extract');
  
  // Single entry extraction state
  const [sampleText, setSampleText] = useState("");
  const [extractionResult, setExtractionResult] = useState<TagExtractionResult | null>(null);
  const [isExtracting, setIsExtracting] = useState(false);
  const [selectedSuggestions, setSelectedSuggestions] = useState<Set<string>>(new Set());
  
  // Bulk extraction state
  const [bulkResults, setBulkResults] = useState<BulkTagResult[]>([]);
  const [isBulkProcessing, setIsBulkProcessing] = useState(false);
  const [bulkProgress, setBulkProgress] = useState(0);
  
  // Custom tag creation state
  const [newTagName, setNewTagName] = useState("");
  const [newTagDescription, setNewTagDescription] = useState("");
  const [newTagCategory, setNewTagCategory] = useState("");
  const [newTagAliases, setNewTagAliases] = useState("");

  useEffect(() => {
    loadInitialData();
  }, []);

  const loadInitialData = async () => {
    try {
      setIsLoading(true);
      const [vocabData, statsData] = await Promise.all([
        invoke<ControlledVocabulary>("get_vocabulary"),
        invoke<TagStatistic[]>("get_tag_statistics")
      ]);
      
      setVocabulary(vocabData);
      setStatistics(statsData);
      
      // Set sample text for demonstration
      setSampleText("Today I had a great meeting at work with my colleagues. We discussed the new project and I'm feeling really excited about the creative possibilities. I want to learn more about the technical aspects and set some goals for the next quarter.");
    } catch (error) {
      console.error("Failed to load tagging data:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const extractTags = async () => {
    if (!sampleText.trim()) return;
    
    try {
      setIsExtracting(true);
      const result = await invoke<TagExtractionResult>("extract_tags_for_entry", {
        entryId: "sample-entry",
        text: sampleText,
        maxTags: 5,
        confidenceThreshold: 0.3
      });
      
      setExtractionResult(result);
      setSelectedSuggestions(new Set());
    } catch (error) {
      console.error("Tag extraction failed:", error);
    } finally {
      setIsExtracting(false);
    }
  };

  const toggleSuggestion = (tag: string) => {
    const newSelected = new Set(selectedSuggestions);
    if (newSelected.has(tag)) {
      newSelected.delete(tag);
    } else {
      newSelected.add(tag);
    }
    setSelectedSuggestions(newSelected);
  };

  const applySelectedTags = async () => {
    if (selectedSuggestions.size === 0) return;
    
    try {
      await invoke("update_entry_tags", {
        entryId: "sample-entry",
        tags: Array.from(selectedSuggestions)
      });
      
      alert(`Applied ${selectedSuggestions.size} tags successfully!`);
      setSelectedSuggestions(new Set());
    } catch (error) {
      console.error("Failed to apply tags:", error);
      alert("Failed to apply tags");
    }
  };

  const runBulkExtraction = async () => {
    try {
      setIsBulkProcessing(true);
      setBulkProgress(0);
      
      // Mock entry IDs for demonstration
      const entryIds = Array.from({ length: 10 }, (_, i) => `entry-${i + 1}`);
      
      const results = await invoke<BulkTagResult[]>("bulk_extract_tags", {
        entryIds,
        maxTags: 3,
        confidenceThreshold: 0.5
      });
      
      setBulkResults(results);
      setBulkProgress(100);
    } catch (error) {
      console.error("Bulk extraction failed:", error);
    } finally {
      setIsBulkProcessing(false);
    }
  };

  const createCustomTag = async () => {
    if (!newTagName.trim() || !newTagDescription.trim()) return;
    
    try {
      const aliases = newTagAliases.split(',').map(a => a.trim()).filter(a => a);
      
      await invoke("create_custom_tag", {
        name: newTagName,
        description: newTagDescription,
        category: newTagCategory || "custom",
        aliases
      });
      
      alert(`Created tag "${newTagName}" successfully!`);
      
      // Reset form
      setNewTagName("");
      setNewTagDescription("");
      setNewTagCategory("");
      setNewTagAliases("");
      
      // Reload vocabulary
      loadInitialData();
    } catch (error) {
      console.error("Failed to create tag:", error);
      alert("Failed to create tag");
    }
  };

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) return "text-green-600 bg-green-100";
    if (confidence >= 0.6) return "text-yellow-600 bg-yellow-100";
    return "text-red-600 bg-red-100";
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case "general": return <Tag className="w-4 h-4" />;
      case "activities": return <Zap className="w-4 h-4" />;
      case "mental": return <Brain className="w-4 h-4" />;
      case "planning": return <Target className="w-4 h-4" />;
      case "social": return <Tag className="w-4 h-4" />;
      case "lifestyle": return <Tag className="w-4 h-4" />;
      case "development": return <Lightbulb className="w-4 h-4" />;
      default: return <Tag className="w-4 h-4" />;
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <Brain className="w-8 h-8 mx-auto mb-2 animate-pulse" />
          <p>Loading tagging system...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">Auto-Tagging</h2>
          <p className="text-muted-foreground">AI-powered tag extraction with controlled vocabulary</p>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant="outline" className="flex items-center gap-1">
            <Tag className="w-3 h-3" />
            {vocabulary?.tags.length || 0} tags
          </Badge>
        </div>
      </div>

      <Tabs value={currentTab} onValueChange={(value: any) => setCurrentTab(value)}>
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="extract" className="flex items-center gap-2">
            <Wand2 className="w-4 h-4" />
            Extract
          </TabsTrigger>
          <TabsTrigger value="vocabulary" className="flex items-center gap-2">
            <Tag className="w-4 h-4" />
            Vocabulary
          </TabsTrigger>
          <TabsTrigger value="statistics" className="flex items-center gap-2">
            <BarChart3 className="w-4 h-4" />
            Statistics
          </TabsTrigger>
          <TabsTrigger value="bulk" className="flex items-center gap-2">
            <Zap className="w-4 h-4" />
            Bulk Process
          </TabsTrigger>
        </TabsList>

        {/* Tag Extraction Tab */}
        <TabsContent value="extract" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Wand2 className="w-5 h-5" />
                Tag Extraction
              </CardTitle>
              <CardDescription>
                Extract relevant tags from journal entry text using AI
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="sample-text">Sample Text</Label>
                <textarea
                  id="sample-text"
                  className="w-full h-32 p-3 border rounded-md resize-none"
                  value={sampleText}
                  onChange={(e) => setSampleText(e.target.value)}
                  placeholder="Enter journal entry text to extract tags..."
                />
              </div>
              
              <Button 
                onClick={extractTags} 
                disabled={isExtracting || !sampleText.trim()}
                className="w-full"
              >
                {isExtracting ? (
                  <>
                    <Clock className="w-4 h-4 mr-2 animate-spin" />
                    Extracting Tags...
                  </>
                ) : (
                  <>
                    <Brain className="w-4 h-4 mr-2" />
                    Extract Tags
                  </>
                )}
              </Button>

              {extractionResult && (
                <div className="space-y-4 pt-4 border-t">
                  <div className="flex items-center justify-between">
                    <h4 className="font-medium">Tag Suggestions</h4>
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <Clock className="w-3 h-3" />
                      {extractionResult.processing_time_ms}ms
                      <Badge variant="outline" size="sm">
                        {extractionResult.model_used}
                      </Badge>
                    </div>
                  </div>
                  
                  <div className="space-y-2">
                    {extractionResult.suggestions.map((suggestion) => (
                      <div
                        key={suggestion.tag}
                        className={`p-3 border rounded-lg cursor-pointer transition-colors ${
                          selectedSuggestions.has(suggestion.tag)
                            ? 'border-primary bg-primary/5'
                            : 'hover:bg-muted/50'
                        }`}
                        onClick={() => toggleSuggestion(suggestion.tag)}
                      >
                        <div className="flex items-center justify-between mb-2">
                          <div className="flex items-center gap-2">
                            <Badge variant="secondary">{suggestion.tag}</Badge>
                            <Badge 
                              variant="outline" 
                              size="sm"
                              className={getConfidenceColor(suggestion.confidence)}
                            >
                              {(suggestion.confidence * 100).toFixed(0)}%
                            </Badge>
                          </div>
                          {selectedSuggestions.has(suggestion.tag) ? (
                            <Check className="w-4 h-4 text-primary" />
                          ) : (
                            <Plus className="w-4 h-4 text-muted-foreground" />
                          )}
                        </div>
                        <p className="text-sm text-muted-foreground mb-1">
                          {suggestion.reasoning}
                        </p>
                        {suggestion.text_spans.length > 0 && (
                          <div className="flex flex-wrap gap-1">
                            {suggestion.text_spans.map((span, i) => (
                              <Badge key={i} variant="outline" size="sm">
                                "{span}"
                              </Badge>
                            ))}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                  
                  {selectedSuggestions.size > 0 && (
                    <Button onClick={applySelectedTags} className="w-full">
                      Apply {selectedSuggestions.size} Selected Tags
                    </Button>
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Vocabulary Management Tab */}
        <TabsContent value="vocabulary" className="space-y-4">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {/* Existing Vocabulary */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Tag className="w-5 h-5" />
                  Controlled Vocabulary
                </CardTitle>
                <CardDescription>
                  Manage your tag vocabulary and aliases
                </CardDescription>
              </CardHeader>
              <CardContent>
                {vocabulary && (
                  <ScrollArea className="h-96">
                    <div className="space-y-3">
                      {vocabulary.tags.map((tag) => (
                        <div key={tag.name} className="border rounded-lg p-3">
                          <div className="flex items-center justify-between mb-2">
                            <div className="flex items-center gap-2">
                              {getCategoryIcon(tag.category)}
                              <Badge variant="default">{tag.name}</Badge>
                              <Badge variant="outline" size="sm">
                                {tag.category}
                              </Badge>
                            </div>
                          </div>
                          <p className="text-sm text-muted-foreground mb-2">
                            {tag.description}
                          </p>
                          {tag.aliases.length > 0 && (
                            <div className="flex flex-wrap gap-1 mb-2">
                              <span className="text-xs text-muted-foreground">Aliases:</span>
                              {tag.aliases.map((alias) => (
                                <Badge key={alias} variant="secondary" size="sm">
                                  {alias}
                                </Badge>
                              ))}
                            </div>
                          )}
                          {tag.examples.length > 0 && (
                            <div className="flex flex-wrap gap-1">
                              <span className="text-xs text-muted-foreground">Examples:</span>
                              {tag.examples.map((example, i) => (
                                <Badge key={i} variant="outline" size="sm">
                                  {example}
                                </Badge>
                              ))}
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  </ScrollArea>
                )}
              </CardContent>
            </Card>

            {/* Create Custom Tag */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Plus className="w-5 h-5" />
                  Create Custom Tag
                </CardTitle>
                <CardDescription>
                  Add new tags to your vocabulary
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="tag-name">Tag Name</Label>
                  <Input
                    id="tag-name"
                    value={newTagName}
                    onChange={(e) => setNewTagName(e.target.value)}
                    placeholder="e.g., productivity"
                  />
                </div>
                
                <div className="space-y-2">
                  <Label htmlFor="tag-description">Description</Label>
                  <textarea
                    id="tag-description"
                    className="w-full h-20 p-3 border rounded-md resize-none"
                    value={newTagDescription}
                    onChange={(e) => setNewTagDescription(e.target.value)}
                    placeholder="Describe what this tag represents..."
                  />
                </div>
                
                <div className="space-y-2">
                  <Label htmlFor="tag-category">Category</Label>
                  <Input
                    id="tag-category"
                    value={newTagCategory}
                    onChange={(e) => setNewTagCategory(e.target.value)}
                    placeholder="e.g., work, personal, activities"
                  />
                </div>
                
                <div className="space-y-2">
                  <Label htmlFor="tag-aliases">Aliases (comma-separated)</Label>
                  <Input
                    id="tag-aliases"
                    value={newTagAliases}
                    onChange={(e) => setNewTagAliases(e.target.value)}
                    placeholder="e.g., efficient, effective, organized"
                  />
                </div>
                
                <Button 
                  onClick={createCustomTag}
                  disabled={!newTagName.trim() || !newTagDescription.trim()}
                  className="w-full"
                >
                  <Plus className="w-4 h-4 mr-2" />
                  Create Tag
                </Button>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Statistics Tab */}
        <TabsContent value="statistics" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <BarChart3 className="w-5 h-5" />
                Tag Usage Statistics
              </CardTitle>
              <CardDescription>
                Analyze your tagging patterns and most used tags
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {statistics.map((stat) => (
                  <div key={stat.tag} className="flex items-center justify-between p-3 border rounded-lg">
                    <div className="flex items-center gap-3">
                      <Badge variant="secondary">{stat.tag}</Badge>
                      <div className="text-sm text-muted-foreground">
                        {stat.count} entries ({stat.percentage.toFixed(1)}%)
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <div className="text-xs text-muted-foreground">
                        Last used: {stat.recent_usage}
                      </div>
                      <div className="w-24 bg-muted rounded-full h-2">
                        <div 
                          className="bg-primary h-2 rounded-full"
                          style={{ width: `${stat.percentage}%` }}
                        />
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Bulk Processing Tab */}
        <TabsContent value="bulk" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Zap className="w-5 h-5" />
                Bulk Tag Extraction
              </CardTitle>
              <CardDescription>
                Process multiple entries at once for tag suggestions
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center gap-4">
                <Button 
                  onClick={runBulkExtraction}
                  disabled={isBulkProcessing}
                  className="flex-1"
                >
                  {isBulkProcessing ? (
                    <>
                      <Clock className="w-4 h-4 mr-2 animate-spin" />
                      Processing...
                    </>
                  ) : (
                    <>
                      <Zap className="w-4 h-4 mr-2" />
                      Run Bulk Extraction
                    </>
                  )}
                </Button>
              </div>
              
              {isBulkProcessing && (
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span>Progress</span>
                    <span>{bulkProgress}%</span>
                  </div>
                  <Progress value={bulkProgress} />
                </div>
              )}
              
              {bulkResults.length > 0 && (
                <div className="space-y-4 pt-4 border-t">
                  <h4 className="font-medium">Bulk Processing Results</h4>
                  <ScrollArea className="h-64">
                    <div className="space-y-2">
                      {bulkResults.map((result) => (
                        <div key={result.entry_id} className="border rounded-lg p-3">
                          <div className="flex items-center justify-between mb-2">
                            <Badge variant="outline">{result.entry_id}</Badge>
                            {result.success ? (
                              <Check className="w-4 h-4 text-green-600" />
                            ) : (
                              <X className="w-4 h-4 text-red-600" />
                            )}
                          </div>
                          
                          {result.success && result.suggestions ? (
                            <div className="flex flex-wrap gap-1">
                              {result.suggestions.map((suggestion) => (
                                <Badge key={suggestion.tag} variant="secondary" size="sm">
                                  {suggestion.tag} ({(suggestion.confidence * 100).toFixed(0)}%)
                                </Badge>
                              ))}
                            </div>
                          ) : result.error ? (
                            <div className="flex items-center gap-2 text-sm text-red-600">
                              <AlertCircle className="w-3 h-3" />
                              {result.error}
                            </div>
                          ) : null}
                        </div>
                      ))}
                    </div>
                  </ScrollArea>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}




