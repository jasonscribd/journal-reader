-- CreateTable
CREATE TABLE "Entry" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "title" TEXT,
    "body" TEXT NOT NULL,
    "entryDate" DATETIME NOT NULL,
    "entryTimezone" TEXT NOT NULL DEFAULT 'UTC',
    "sourcePath" TEXT NOT NULL,
    "sourceType" TEXT NOT NULL,
    "textHash" TEXT NOT NULL,
    "embedding" BLOB,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL,
    "sentiment" REAL,
    "language" TEXT
);

-- CreateTable
CREATE TABLE "Tag" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "name" TEXT NOT NULL,
    "parentId" TEXT,
    "description" TEXT,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL,
    CONSTRAINT "Tag_parentId_fkey" FOREIGN KEY ("parentId") REFERENCES "Tag" ("id") ON DELETE SET NULL ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "Alias" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "tagId" TEXT NOT NULL,
    "aliasText" TEXT NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "Alias_tagId_fkey" FOREIGN KEY ("tagId") REFERENCES "Tag" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "EntryTag" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "entryId" TEXT NOT NULL,
    "tagId" TEXT NOT NULL,
    CONSTRAINT "EntryTag_entryId_fkey" FOREIGN KEY ("entryId") REFERENCES "Entry" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "EntryTag_tagId_fkey" FOREIGN KEY ("tagId") REFERENCES "Tag" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "ImportJob" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "rootPath" TEXT NOT NULL,
    "status" TEXT NOT NULL,
    "totalFiles" INTEGER NOT NULL DEFAULT 0,
    "processed" INTEGER NOT NULL DEFAULT 0,
    "succeeded" INTEGER NOT NULL DEFAULT 0,
    "failed" INTEGER NOT NULL DEFAULT 0,
    "errorLog" TEXT,
    "startedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "finishedAt" DATETIME
);

-- CreateTable
CREATE TABLE "Conversation" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "title" TEXT NOT NULL,
    "systemPrompt" TEXT,
    "provider" TEXT NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL
);

-- CreateTable
CREATE TABLE "Message" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "conversationId" TEXT NOT NULL,
    "role" TEXT NOT NULL,
    "content" TEXT NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "Message_conversationId_fkey" FOREIGN KEY ("conversationId") REFERENCES "Conversation" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "Citation" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "messageId" TEXT NOT NULL,
    "entryId" TEXT NOT NULL,
    "span" TEXT,
    CONSTRAINT "Citation_messageId_fkey" FOREIGN KEY ("messageId") REFERENCES "Message" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "Citation_entryId_fkey" FOREIGN KEY ("entryId") REFERENCES "Entry" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "Setting" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "key" TEXT NOT NULL,
    "value" TEXT NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL
);

-- CreateTable
CREATE TABLE "SettingAudit" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "settingId" TEXT NOT NULL,
    "oldValue" TEXT,
    "newValue" TEXT NOT NULL,
    "changedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "SettingAudit_settingId_fkey" FOREIGN KEY ("settingId") REFERENCES "Setting" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "IndexState" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "embeddingModel" TEXT NOT NULL,
    "version" TEXT NOT NULL,
    "lastBuild" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "Entry_textHash_key" ON "Entry"("textHash");

-- CreateIndex
CREATE INDEX "Entry_entryDate_idx" ON "Entry"("entryDate");

-- CreateIndex
CREATE INDEX "Entry_textHash_idx" ON "Entry"("textHash");

-- CreateIndex
CREATE INDEX "Entry_sourcePath_idx" ON "Entry"("sourcePath");

-- CreateIndex
CREATE UNIQUE INDEX "Tag_name_key" ON "Tag"("name");

-- CreateIndex
CREATE UNIQUE INDEX "Alias_aliasText_key" ON "Alias"("aliasText");

-- CreateIndex
CREATE UNIQUE INDEX "EntryTag_entryId_tagId_key" ON "EntryTag"("entryId", "tagId");

-- CreateIndex
CREATE INDEX "ImportJob_status_idx" ON "ImportJob"("status");

-- CreateIndex
CREATE INDEX "ImportJob_startedAt_idx" ON "ImportJob"("startedAt");

-- CreateIndex
CREATE INDEX "Message_conversationId_idx" ON "Message"("conversationId");

-- CreateIndex
CREATE INDEX "Citation_messageId_idx" ON "Citation"("messageId");

-- CreateIndex
CREATE INDEX "Citation_entryId_idx" ON "Citation"("entryId");

-- CreateIndex
CREATE UNIQUE INDEX "Setting_key_key" ON "Setting"("key");

-- CreateIndex
CREATE INDEX "SettingAudit_settingId_idx" ON "SettingAudit"("settingId");

-- CreateIndex
CREATE INDEX "SettingAudit_changedAt_idx" ON "SettingAudit"("changedAt");
