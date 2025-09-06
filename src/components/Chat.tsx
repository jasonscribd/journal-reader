import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { 
  MessageCircle, 
  Send, 
  Bot, 
  User, 
  Clock, 
  ExternalLink, 
  Trash2, 
  Plus, 
  Lightbulb,
  Settings,
  Quote,
  Calendar,
  Tag,
  Brain,
  Zap,
  AlertCircle,
  CheckCircle,
  Copy,
  RefreshCw
} from "lucide-react";

interface Citation {
  entry_id: string;
  entry_title?: string;
  entry_date: string;
  snippet: string;
  relevance_score: number;
  citation_number: number;
}

interface ContextEntry {
  entry_id: string;
  title?: string;
  body: string;
  entry_date: string;
  tags: string[];
  relevance_score: number;
  snippet: string;
}

interface RagResponse {
  answer: string;
  citations: Citation[];
  context_used: ContextEntry[];
  confidence: number;
  processing_time_ms: number;
  model_used: string;
  conversation_id: string;
  message_id: string;
}

interface ConversationMessage {
  message_id: string;
  role: string;
  content: string;
  citations?: Citation[];
  timestamp: string;
}

interface ConversationSummary {
  conversation_id: string;
  title: string;
  last_message: string;
  message_count: number;
  created_at: string;
  updated_at: string;
}

export function Chat() {
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [currentConversation, setCurrentConversation] = useState<string | null>(null);
  const [messages, setMessages] = useState<ConversationMessage[]>([]);
  const [currentQuestion, setCurrentQuestion] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [suggestedQuestions, setSuggestedQuestions] = useState<string[]>([]);
  const [selectedProvider, setSelectedProvider] = useState("ollama");
  const [selectedModel, setSelectedModel] = useState("llama3.1:8b");
  const [maxContextEntries, setMaxContextEntries] = useState(5);
  const [contextTags, setContextTags] = useState<string[]>([]);
  const [showSettings, setShowSettings] = useState(false);
  const [currentTab, setCurrentTab] = useState<'chat' | 'conversations' | 'settings'>('chat');
  
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    loadInitialData();
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  useEffect(() => {
    if (currentConversation) {
      loadConversationHistory(currentConversation);
    }
  }, [currentConversation]);

  const loadInitialData = async () => {
    try {
      const [conversationsList, suggestions] = await Promise.all([
        invoke<ConversationSummary[]>("get_conversations_list"),
        invoke<string[]>("get_suggested_questions")
      ]);
      
      setConversations(conversationsList);
      setSuggestedQuestions(suggestions);
      
      // Load the most recent conversation if available
      if (conversationsList.length > 0) {
        setCurrentConversation(conversationsList[0].conversation_id);
      }
    } catch (error) {
      console.error("Failed to load chat data:", error);
    }
  };

  const loadConversationHistory = async (conversationId: string) => {
    try {
      const history = await invoke<ConversationMessage[]>("get_conversation_history", {
        conversationId
      });
      setMessages(history);
    } catch (error) {
      console.error("Failed to load conversation history:", error);
      setMessages([]);
    }
  };

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  const askQuestion = async (question: string) => {
    if (!question.trim()) return;
    
    try {
      setIsLoading(true);
      
      // Add user message immediately
      const userMessage: ConversationMessage = {
        message_id: `user-${Date.now()}`,
        role: "user",
        content: question,
        timestamp: new Date().toISOString(),
      };
      
      setMessages(prev => [...prev, userMessage]);
      setCurrentQuestion("");
      
      // Get AI response
      const response = await invoke<RagResponse>("ask_question", {
        question,
        conversationId: currentConversation,
        maxContextEntries,
        contextDateRange: null,
        contextTags: contextTags.length > 0 ? contextTags : null,
        provider: selectedProvider,
        model: selectedModel,
      });
      
      // Add AI response
      const aiMessage: ConversationMessage = {
        message_id: response.message_id,
        role: "assistant",
        content: response.answer,
        citations: response.citations,
        timestamp: new Date().toISOString(),
      };
      
      setMessages(prev => [...prev, aiMessage]);
      
      // Update current conversation ID if this was a new conversation
      if (!currentConversation) {
        setCurrentConversation(response.conversation_id);
        // Reload conversations list to include the new one
        loadInitialData();
      }
      
    } catch (error) {
      console.error("Failed to ask question:", error);
      
      // Add error message
      const errorMessage: ConversationMessage = {
        message_id: `error-${Date.now()}`,
        role: "assistant",
        content: "I'm sorry, I encountered an error while processing your question. Please try again.",
        timestamp: new Date().toISOString(),
      };
      
      setMessages(prev => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      askQuestion(currentQuestion);
    }
  };

  const startNewConversation = () => {
    setCurrentConversation(null);
    setMessages([]);
    setCurrentQuestion("");
    inputRef.current?.focus();
  };

  const deleteConversation = async (conversationId: string) => {
    try {
      await invoke("delete_conversation", { conversationId });
      
      // Remove from local state
      setConversations(prev => prev.filter(c => c.conversation_id !== conversationId));
      
      // If this was the current conversation, clear it
      if (currentConversation === conversationId) {
        startNewConversation();
      }
    } catch (error) {
      console.error("Failed to delete conversation:", error);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) return "text-green-600";
    if (confidence >= 0.6) return "text-yellow-600";
    return "text-red-600";
  };

  const getProviderIcon = (provider: string) => {
    switch (provider) {
      case "openai": return <Zap className="w-4 h-4" />;
      case "ollama": return <Brain className="w-4 h-4" />;
      default: return <Bot className="w-4 h-4" />;
    }
  };

  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <div className="w-80 border-r bg-card flex flex-col">
        <div className="p-4 border-b">
          <div className="flex items-center justify-between mb-4">
            <h3 className="font-semibold">Journal Q&A</h3>
            <Button size="sm" onClick={startNewConversation}>
              <Plus className="w-4 h-4 mr-1" />
              New Chat
            </Button>
          </div>
          
          <Tabs value={currentTab} onValueChange={(value: any) => setCurrentTab(value)}>
            <TabsList className="grid w-full grid-cols-3">
              <TabsTrigger value="chat" className="text-xs">Chat</TabsTrigger>
              <TabsTrigger value="conversations" className="text-xs">History</TabsTrigger>
              <TabsTrigger value="settings" className="text-xs">Settings</TabsTrigger>
            </TabsList>
          </Tabs>
        </div>

        <div className="flex-1 overflow-hidden">
          {currentTab === 'chat' && (
            <div className="p-4 space-y-4">
              <div>
                <h4 className="text-sm font-medium mb-2 flex items-center gap-2">
                  <Lightbulb className="w-4 h-4" />
                  Suggested Questions
                </h4>
                <div className="space-y-2">
                  {suggestedQuestions.slice(0, 5).map((question, index) => (
                    <Button
                      key={index}
                      variant="outline"
                      size="sm"
                      className="w-full text-left justify-start h-auto p-2 text-xs"
                      onClick={() => setCurrentQuestion(question)}
                    >
                      {question}
                    </Button>
                  ))}
                </div>
              </div>
            </div>
          )}

          {currentTab === 'conversations' && (
            <ScrollArea className="flex-1">
              <div className="p-4 space-y-2">
                {conversations.map((conversation) => (
                  <div
                    key={conversation.conversation_id}
                    className={`p-3 rounded-lg border cursor-pointer transition-colors ${
                      currentConversation === conversation.conversation_id
                        ? 'border-primary bg-primary/5'
                        : 'hover:bg-muted/50'
                    }`}
                    onClick={() => setCurrentConversation(conversation.conversation_id)}
                  >
                    <div className="flex items-start justify-between mb-1">
                      <h4 className="font-medium text-sm line-clamp-1">
                        {conversation.title}
                      </h4>
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-6 w-6 p-0 opacity-0 group-hover:opacity-100"
                        onClick={(e) => {
                          e.stopPropagation();
                          deleteConversation(conversation.conversation_id);
                        }}
                      >
                        <Trash2 className="w-3 h-3" />
                      </Button>
                    </div>
                    <p className="text-xs text-muted-foreground line-clamp-2 mb-2">
                      {conversation.last_message}
                    </p>
                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                      <span>{conversation.message_count} messages</span>
                      <span>{formatDate(conversation.updated_at)}</span>
                    </div>
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}

          {currentTab === 'settings' && (
            <div className="p-4 space-y-4">
              <div className="space-y-2">
                <Label htmlFor="provider">AI Provider</Label>
                <Select value={selectedProvider} onValueChange={setSelectedProvider}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="ollama">
                      <div className="flex items-center gap-2">
                        <Brain className="w-4 h-4" />
                        Ollama (Local)
                      </div>
                    </SelectItem>
                    <SelectItem value="openai">
                      <div className="flex items-center gap-2">
                        <Zap className="w-4 h-4" />
                        OpenAI (Cloud)
                      </div>
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="model">Model</Label>
                <Select value={selectedModel} onValueChange={setSelectedModel}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {selectedProvider === "ollama" ? (
                      <>
                        <SelectItem value="llama3.1:8b">Llama 3.1 8B</SelectItem>
                        <SelectItem value="phi3:mini">Phi-3 Mini</SelectItem>
                        <SelectItem value="gemma2:9b">Gemma 2 9B</SelectItem>
                      </>
                    ) : (
                      <>
                        <SelectItem value="gpt-4o-mini">GPT-4o Mini</SelectItem>
                        <SelectItem value="gpt-4o">GPT-4o</SelectItem>
                        <SelectItem value="gpt-3.5-turbo">GPT-3.5 Turbo</SelectItem>
                      </>
                    )}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="context-entries">Max Context Entries</Label>
                <Select value={maxContextEntries.toString()} onValueChange={(value) => setMaxContextEntries(parseInt(value))}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="3">3 entries</SelectItem>
                    <SelectItem value="5">5 entries</SelectItem>
                    <SelectItem value="10">10 entries</SelectItem>
                    <SelectItem value="15">15 entries</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Main Chat Area */}
      <div className="flex-1 flex flex-col">
        {/* Chat Header */}
        <div className="border-b bg-card px-6 py-4">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-xl font-semibold">
                {currentConversation ? "Conversation" : "New Conversation"}
              </h2>
              <p className="text-sm text-muted-foreground">
                Ask questions about your journal entries
              </p>
            </div>
            <div className="flex items-center gap-2">
              {getProviderIcon(selectedProvider)}
              <Badge variant="outline">{selectedModel}</Badge>
            </div>
          </div>
        </div>

        {/* Messages */}
        <ScrollArea className="flex-1 p-6">
          <div className="space-y-6 max-w-4xl mx-auto">
            {messages.length === 0 && (
              <div className="text-center py-12">
                <MessageCircle className="w-12 h-12 mx-auto mb-4 text-muted-foreground" />
                <h3 className="text-lg font-medium mb-2">Start a conversation</h3>
                <p className="text-muted-foreground mb-4">
                  Ask me anything about your journal entries. I'll search through your entries and provide answers with citations.
                </p>
                <div className="flex flex-wrap gap-2 justify-center">
                  {suggestedQuestions.slice(0, 3).map((question, index) => (
                    <Button
                      key={index}
                      variant="outline"
                      size="sm"
                      onClick={() => setCurrentQuestion(question)}
                    >
                      {question}
                    </Button>
                  ))}
                </div>
              </div>
            )}

            {messages.map((message) => (
              <div
                key={message.message_id}
                className={`flex gap-4 ${
                  message.role === 'user' ? 'justify-end' : 'justify-start'
                }`}
              >
                {message.role === 'assistant' && (
                  <div className="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0">
                    <Bot className="w-4 h-4" />
                  </div>
                )}
                
                <div className={`max-w-3xl ${message.role === 'user' ? 'order-first' : ''}`}>
                  <div
                    className={`rounded-lg p-4 ${
                      message.role === 'user'
                        ? 'bg-primary text-primary-foreground ml-12'
                        : 'bg-muted'
                    }`}
                  >
                    <div className="prose prose-sm max-w-none">
                      {message.content}
                    </div>
                    
                    {message.role === 'assistant' && (
                      <div className="flex items-center justify-between mt-3 pt-3 border-t border/50">
                        <div className="flex items-center gap-2 text-xs text-muted-foreground">
                          <Clock className="w-3 h-3" />
                          {formatDate(message.timestamp)}
                        </div>
                        <Button
                          size="sm"
                          variant="ghost"
                          className="h-6 px-2"
                          onClick={() => copyToClipboard(message.content)}
                        >
                          <Copy className="w-3 h-3" />
                        </Button>
                      </div>
                    )}
                  </div>
                  
                  {/* Citations */}
                  {message.citations && message.citations.length > 0 && (
                    <div className="mt-3 space-y-2">
                      <h4 className="text-sm font-medium flex items-center gap-2">
                        <Quote className="w-4 h-4" />
                        Sources ({message.citations.length})
                      </h4>
                      <div className="space-y-2">
                        {message.citations.map((citation) => (
                          <div
                            key={citation.citation_number}
                            className="border rounded-lg p-3 bg-card"
                          >
                            <div className="flex items-start justify-between mb-2">
                              <div className="flex items-center gap-2">
                                <Badge variant="secondary" size="sm">
                                  [{citation.citation_number}]
                                </Badge>
                                <span className="text-sm font-medium">
                                  {citation.entry_title || "Untitled Entry"}
                                </span>
                              </div>
                              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                                <Calendar className="w-3 h-3" />
                                {formatDate(citation.entry_date)}
                              </div>
                            </div>
                            <p className="text-sm text-muted-foreground mb-2">
                              "{citation.snippet}"
                            </p>
                            <div className="flex items-center justify-between">
                              <Badge
                                variant="outline"
                                size="sm"
                                className={getConfidenceColor(citation.relevance_score)}
                              >
                                {(citation.relevance_score * 100).toFixed(0)}% relevant
                              </Badge>
                              <Button size="sm" variant="ghost" className="h-6 px-2">
                                <ExternalLink className="w-3 h-3 mr-1" />
                                View Entry
                              </Button>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
                
                {message.role === 'user' && (
                  <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center flex-shrink-0">
                    <User className="w-4 h-4" />
                  </div>
                )}
              </div>
            ))}
            
            {isLoading && (
              <div className="flex gap-4 justify-start">
                <div className="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0">
                  <Bot className="w-4 h-4" />
                </div>
                <div className="bg-muted rounded-lg p-4 max-w-3xl">
                  <div className="flex items-center gap-2">
                    <RefreshCw className="w-4 h-4 animate-spin" />
                    <span className="text-sm">Thinking...</span>
                  </div>
                </div>
              </div>
            )}
            
            <div ref={messagesEndRef} />
          </div>
        </ScrollArea>

        {/* Input Area */}
        <div className="border-t bg-card p-6">
          <div className="max-w-4xl mx-auto">
            <div className="flex gap-3">
              <div className="flex-1">
                <Input
                  ref={inputRef}
                  value={currentQuestion}
                  onChange={(e) => setCurrentQuestion(e.target.value)}
                  onKeyPress={handleKeyPress}
                  placeholder="Ask a question about your journal entries..."
                  disabled={isLoading}
                  className="text-base"
                />
              </div>
              <Button
                onClick={() => askQuestion(currentQuestion)}
                disabled={isLoading || !currentQuestion.trim()}
                size="lg"
              >
                <Send className="w-4 h-4" />
              </Button>
            </div>
            
            <div className="flex items-center justify-between mt-2 text-xs text-muted-foreground">
              <span>Press Enter to send, Shift+Enter for new line</span>
              <span>Using {maxContextEntries} context entries</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

