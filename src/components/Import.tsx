import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { 
  FileText, 
  FolderOpen, 
  Calendar, 
  Upload, 
  CheckCircle, 
  Clock,
  AlertCircle,
  Trash2
} from "lucide-react";

interface FileImportItem {
  path: string;
  title: string | null;
  size_bytes: number;
  file_type: string;
  suggested_date: string | null;
}

interface FileWithDate {
  path: string;
  entry_date: string;
  entry_timezone: string;
}

interface ImportResult {
  imported: number;
  failed: number;
  errors?: string[];
}



export function Import() {
  const [currentStep, setCurrentStep] = useState<'select' | 'configure' | 'import' | 'complete'>('select');
  const [selectedFiles, setSelectedFiles] = useState<FileImportItem[]>([]);
  const [filesWithDates, setFilesWithDates] = useState<FileWithDate[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [bulkMonth, setBulkMonth] = useState<number>(new Date().getMonth() + 1); // 1-12
  const [bulkYear, setBulkYear] = useState<number>(new Date().getFullYear());
  const [bulkTimezone, setBulkTimezone] = useState<string>('UTC');





  // No background job; import is synchronous

  const selectFiles = async () => {
    try {
      console.log("Opening file dialog...");
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: "Journal Files",
            extensions: ["txt", "doc", "docx", "gdoc"]
          }
        ]
      });

      console.log("Dialog result:", selected);

      if (selected && Array.isArray(selected)) {
        console.log("Selected files:", selected);
        await scanFiles(selected);
      } else if (selected === null) {
        console.log("User cancelled file selection");
      } else {
        console.log("Unexpected dialog result:", selected);
      }
    } catch (error) {
      console.error("Failed to select files:", error);
      alert(`Error opening file dialog: ${error}`);
    }
  };

  const selectFolder = async () => {
    try {
      console.log("Opening folder dialog...");
      const selected = await open({
        directory: true
      });

      console.log("Folder dialog result:", selected);

      if (selected && typeof selected === 'string') {
        console.log("Selected folder:", selected);
        await scanFiles([selected]);
      } else if (selected === null) {
        console.log("User cancelled folder selection");
      } else {
        console.log("Unexpected folder dialog result:", selected);
      }
    } catch (error) {
      console.error("Failed to select folder:", error);
      alert(`Error opening folder dialog: ${error}`);
    }
  };

  const scanFiles = async (paths: string[]) => {
    try {
      setIsScanning(true);
      const files = await invoke<FileImportItem[]>("scan_import_files", { paths });
      setSelectedFiles(files);
      
      if (files.length > 0) {
        setCurrentStep('configure');
        // Month and year are already initialized with current values
      }
    } catch (error) {
      console.error("Failed to scan files:", error);
      alert(`Failed to scan files: ${error}`);
    } finally {
      setIsScanning(false);
    }
  };

  const applyBulkDate = () => {
    if (!bulkMonth || !bulkYear) return;
    
    // Create date for first day of selected month/year at noon
    const dateTime = new Date(bulkYear, bulkMonth - 1, 1, 12, 0, 0).toISOString();
    
    const updated = selectedFiles.map(file => ({
      path: file.path,
      entry_date: dateTime,
      entry_timezone: bulkTimezone,
    }));
    
    setFilesWithDates(updated);
  };

  const updateFileDate = (index: number, month: number, year: number) => {
    const dateTime = new Date(year, month - 1, 1, 12, 0, 0).toISOString();
    
    setFilesWithDates(prev => {
      const updated = [...prev];
      updated[index] = {
        ...updated[index],
        entry_date: dateTime,
      };
      return updated;
    });
  };

  const removeFile = (index: number) => {
    setSelectedFiles(prev => prev.filter((_, i) => i !== index));
    setFilesWithDates(prev => prev.filter((_, i) => i !== index));
  };

  const startImport = async () => {
    if (filesWithDates.length === 0) {
      alert("Please configure dates for all files first.");
      return;
    }

    try {
      setIsImporting(true);
      setCurrentStep('import');
      
      const res = await invoke<ImportResult>("import_files_with_dates", { files: filesWithDates });
      setResult(res);
      setIsImporting(false);
      setCurrentStep('complete');
    } catch (error) {
      console.error("Failed to start import:", error);
      alert(`Failed to start import: ${error}`);
      setIsImporting(false);
    }
  };

  const resetImport = () => {
    setCurrentStep('select');
    setSelectedFiles([]);
    setFilesWithDates([]);
    setResult(null);
    setIsImporting(false);
    setBulkMonth(new Date().getMonth() + 1);
    setBulkYear(new Date().getFullYear());
  };

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };



  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">Import Files</h2>
          <p className="text-muted-foreground">Import local files into your journal</p>
        </div>
        {currentStep !== 'select' && (
          <Button onClick={resetImport} variant="outline">
            Start Over
          </Button>
        )}
      </div>

      {/* Step Indicator */}
      <div className="flex items-center space-x-4">
        {[
          { key: 'select', label: 'Select Files', icon: FolderOpen },
          { key: 'configure', label: 'Configure Dates', icon: Calendar },
          { key: 'import', label: 'Import', icon: Upload },
          { key: 'complete', label: 'Complete', icon: CheckCircle },
        ].map(({ key, label, icon: Icon }, index) => (
          <div key={key} className="flex items-center">
            <div className={`flex items-center gap-2 px-3 py-2 rounded-lg ${
              currentStep === key 
                ? 'bg-primary text-primary-foreground' 
                : index < ['select', 'configure', 'import', 'complete'].indexOf(currentStep)
                ? 'bg-green-100 text-green-800'
                : 'bg-muted text-muted-foreground'
            }`}>
              <Icon className="w-4 h-4" />
              <span className="text-sm font-medium">{label}</span>
            </div>
            {index < 3 && (
              <div className="w-8 h-0.5 bg-muted mx-2" />
            )}
          </div>
        ))}
      </div>


          {/* Step 1: Select Files */}
          {currentStep === 'select' && (
        <Card>
          <CardHeader>
            <CardTitle>Select Files to Import</CardTitle>
            <CardDescription>
              Choose individual files or entire folders containing your journal entries
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <Button
                onClick={selectFiles}
                disabled={isScanning}
                className="h-24 flex-col gap-2"
                variant="outline"
              >
                <FileText className="w-8 h-8" />
                <div className="text-center">
                  <div className="font-medium">Select Files</div>
                  <div className="text-sm text-muted-foreground">Choose individual TXT/DOCX files</div>
                </div>
              </Button>
              
              <Button
                onClick={selectFolder}
                disabled={isScanning}
                className="h-24 flex-col gap-2"
                variant="outline"
              >
                <FolderOpen className="w-8 h-8" />
                <div className="text-center">
                  <div className="font-medium">Select Folder</div>
                  <div className="text-sm text-muted-foreground">Import all files from a folder</div>
                </div>
              </Button>
            </div>

            {isScanning && (
              <div className="flex items-center gap-2 p-4 bg-muted rounded-lg">
                <Clock className="w-4 h-4 animate-spin" />
                <span>Scanning files...</span>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Step 2: Configure Dates */}
      {currentStep === 'configure' && (
        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Configure Entry Dates</CardTitle>
              <CardDescription>
                Set the entry date for each file. This is required and cannot be auto-detected.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="p-4 bg-muted rounded-lg">
                <h4 className="font-medium mb-3">Bulk Date Assignment</h4>
                <div className="flex gap-4 items-end">
                  <div className="flex-1">
                    <Label>Month</Label>
                    <Select value={bulkMonth.toString()} onValueChange={(value) => setBulkMonth(parseInt(value))}>
                      <SelectTrigger>
                        <SelectValue placeholder="Select month" />
                      </SelectTrigger>
                      <SelectContent>
                        {[
                          'January', 'February', 'March', 'April', 'May', 'June',
                          'July', 'August', 'September', 'October', 'November', 'December'
                        ].map((month, index) => (
                          <SelectItem key={index} value={(index + 1).toString()}>{month}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex-1">
                    <Label>Year</Label>
                    <Select value={bulkYear.toString()} onValueChange={(value) => setBulkYear(parseInt(value))}>
                      <SelectTrigger>
                        <SelectValue placeholder="Select year" />
                      </SelectTrigger>
                      <SelectContent>
                        {Array.from({ length: 50 }, (_, i) => {
                          const year = new Date().getFullYear() + 10 - i;
                          return <SelectItem key={year} value={year.toString()}>{year}</SelectItem>;
                        })}
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex-1">
                    <Label htmlFor="bulk-timezone">Timezone</Label>
                    <Input
                      id="bulk-timezone"
                      value={bulkTimezone}
                      onChange={(e) => setBulkTimezone(e.target.value)}
                      placeholder="UTC"
                    />
                  </div>
                  <Button onClick={applyBulkDate} disabled={!bulkMonth || !bulkYear}>
                    Apply to All
                  </Button>
                </div>
              </div>

              <Separator />

              <div className="space-y-3">
                <h4 className="font-medium">Individual File Configuration</h4>
                <div className="space-y-2 max-h-96 overflow-y-auto">
                  {selectedFiles.map((file, index) => (
                    <div key={file.path} className="flex items-center gap-4 p-3 border rounded-lg">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <Badge variant="secondary">{file.file_type.toUpperCase()}</Badge>
                          <span className="font-medium truncate">
                            {file.title || file.path.split('/').pop()}
                          </span>
                        </div>
                        <div className="text-sm text-muted-foreground truncate">
                          {file.path} • {formatFileSize(file.size_bytes)}
                        </div>
                      </div>
                      
                      <div className="flex items-center gap-2">
                        <div className="flex gap-1">
                          <Select 
                            value={(filesWithDates[index]?.entry_date ? new Date(filesWithDates[index].entry_date).getMonth() + 1 : bulkMonth).toString()}
                            onValueChange={(value) => {
                              const currentYear = filesWithDates[index]?.entry_date ? 
                                new Date(filesWithDates[index].entry_date).getFullYear() : bulkYear;
                              updateFileDate(index, parseInt(value), currentYear);
                            }}
                          >
                            <SelectTrigger className="w-32">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {[
                                'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
                                'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'
                              ].map((month, idx) => (
                                <SelectItem key={idx} value={(idx + 1).toString()}>{month}</SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                          <Select 
                            value={filesWithDates[index]?.entry_date ? new Date(filesWithDates[index].entry_date).getFullYear().toString() : bulkYear.toString()}
                            onValueChange={(value) => {
                              const currentMonth = filesWithDates[index]?.entry_date ? 
                                new Date(filesWithDates[index].entry_date).getMonth() + 1 : bulkMonth;
                              updateFileDate(index, currentMonth, parseInt(value));
                            }}
                          >
                            <SelectTrigger className="w-20">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {Array.from({ length: 30 }, (_, i) => {
                                const year = new Date().getFullYear() + 5 - i;
                                return <SelectItem key={year} value={year.toString()}>{year}</SelectItem>;
                              })}
                            </SelectContent>
                          </Select>
                        </div>
                        <Button
                          onClick={() => removeFile(index)}
                          variant="ghost"
                          size="sm"
                        >
                          <Trash2 className="w-4 h-4" />
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              <div className="flex justify-between">
                <div className="text-sm text-muted-foreground">
                  {selectedFiles.length} files selected
                </div>
                <Button 
                  onClick={startImport}
                  disabled={filesWithDates.length !== selectedFiles.length || 
                           filesWithDates.some(f => !f.entry_date)}
                >
                  Start Import
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Step 3: Import Progress */}
      {currentStep === 'import' && (
        <Card>
          <CardHeader>
            <CardTitle>Importing Files</CardTitle>
            <CardDescription>
              Processing your journal entries...
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center gap-2 p-4 bg-muted rounded-lg">
              <Clock className="w-4 h-4 animate-spin" />
              <span>Working...</span>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Step 4: Complete */}
      {currentStep === 'complete' && result && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              {result.failed === 0 ? (
                <CheckCircle className="w-5 h-5 text-green-600" />
              ) : (
                <AlertCircle className="w-5 h-5 text-yellow-600" />
              )}
              Import Complete
            </CardTitle>
            <CardDescription>
              {result.failed === 0 
                ? "All files were imported successfully!"
                : `${result.imported} files imported, ${result.failed} failed.`
              }
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="p-4 bg-green-50 rounded-lg text-center">
                <div className="text-3xl font-bold text-green-600">{result.imported}</div>
                <div className="text-green-600">Successfully Imported</div>
              </div>
              {result.failed > 0 && (
                <div className="p-4 bg-red-50 rounded-lg text-center">
                  <div className="text-3xl font-bold text-red-600">{result.failed}</div>
                  <div className="text-red-600">Failed</div>
                </div>
              )}
            </div>
            {result.errors && result.errors.length > 0 && (
              <div className="p-3 bg-red-50 border border-red-200 rounded-lg">
                <div className="flex items-center gap-2 mb-2">
                  <AlertCircle className="w-4 h-4 text-red-600" />
                  <span className="font-medium text-red-600">Error Details</span>
                </div>
                <pre className="text-sm text-red-600 whitespace-pre-wrap max-h-32 overflow-y-auto">
                  {result.errors.join("\n")}
                </pre>
              </div>
            )}

            <div className="flex gap-2">
              <Button onClick={resetImport} variant="outline">
                Import More Files
              </Button>
              <Button onClick={() => window.location.reload()}>
                View Timeline
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}


