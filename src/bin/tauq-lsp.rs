use std::collections::HashMap;
use std::sync::Arc;
use tauq::tauq::Parser;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// Document state for tracking open files
#[derive(Debug, Clone)]
struct Document {
    content: String,
    #[allow(dead_code)]
    version: i32,
    schemas: Vec<SchemaInfo>,
}

/// Information about a schema definition
#[derive(Debug, Clone)]
struct SchemaInfo {
    name: String,
    fields: Vec<String>,
    line: u32,
    character: u32,
}

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, Document>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Extract schema definitions from document content
    fn extract_schemas(content: &str) -> Vec<SchemaInfo> {
        let mut schemas = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Check for !def directive
            if let Some(rest) = trimmed.strip_prefix("!def ") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if !parts.is_empty() {
                    let name = parts[0].to_string();
                    let fields: Vec<String> = parts[1..]
                        .iter()
                        .map(|s| s.split(':').next().unwrap_or(s).to_string())
                        .collect();

                    schemas.push(SchemaInfo {
                        name,
                        fields,
                        line: line_num as u32,
                        character: line.find("!def").unwrap_or(0) as u32,
                    });
                }
            }

            // Check for schema block definitions
            // Format: SchemaName field1 field2 (inside !schemas block)
            // This is a simplified check - could be enhanced
        }

        schemas
    }

    /// Generate diagnostics for a document
    async fn generate_diagnostics(&self, _uri: &Url, content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Parse and collect errors
        let mut parser = Parser::new(content);
        if let Err(e) = parser.parse() {
            let diagnostic = Diagnostic {
                range: Range {
                    start: Position {
                        line: (e.span.line.saturating_sub(1)) as u32,
                        character: (e.span.column.saturating_sub(1)) as u32,
                    },
                    end: Position {
                        line: (e.span.line.saturating_sub(1)) as u32,
                        character: (e.span.column) as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("tauq-lsp".to_string()),
                message: e.message.clone(),
                related_information: None,
                tags: None,
                data: None,
            };
            diagnostics.push(diagnostic);
        }

        // Check for undefined schema references
        let schemas = Self::extract_schemas(content);
        let schema_names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(schema_ref) = trimmed.strip_prefix("!use ") {
                let schema_ref = schema_ref.trim();
                if !schema_names.contains(&schema_ref) && !schema_ref.is_empty() {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: line.find("!use").unwrap_or(0) as u32,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: line.len() as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: None,
                        code_description: None,
                        source: Some("tauq-lsp".to_string()),
                        message: format!("Schema '{}' is not defined in this file", schema_ref),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
        }

        diagnostics
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["!".to_string(), " ".to_string()]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::STRING,
                                    SemanticTokenType::NUMBER,
                                    SemanticTokenType::COMMENT,
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::DEFINITION,
                                    SemanticTokenModifier::DECLARATION,
                                ],
                            },
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "tauq-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Tauq Language Server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let content = params.text_document.text.clone();
        let version = params.text_document.version;

        let schemas = Self::extract_schemas(&content);

        {
            let mut docs = self.documents.write().await;
            docs.insert(
                uri.clone(),
                Document {
                    content: content.clone(),
                    version,
                    schemas,
                },
            );
        }

        let diagnostics = self.generate_diagnostics(&uri, &content).await;
        self.client
            .publish_diagnostics(uri, diagnostics, Some(version))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;

        if let Some(change) = params.content_changes.first() {
            let content = change.text.clone();
            let schemas = Self::extract_schemas(&content);

            {
                let mut docs = self.documents.write().await;
                docs.insert(
                    uri.clone(),
                    Document {
                        content: content.clone(),
                        version,
                        schemas,
                    },
                );
            }

            let diagnostics = self.generate_diagnostics(&uri, &content).await;
            self.client
                .publish_diagnostics(uri, diagnostics, Some(version))
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut docs = self.documents.write().await;
        docs.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let lines: Vec<&str> = doc.content.lines().collect();
        let line_idx = position.line as usize;

        if line_idx >= lines.len() {
            return Ok(None);
        }

        let line = lines[line_idx];
        let trimmed = line.trim();

        // Hover over directives
        if trimmed.starts_with("!def") {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "**!def** - Define and activate a schema\n\n```tqn\n!def SchemaName field1 field2 field3:NestedType\n```\n\nThe schema is immediately activated after definition.".to_string(),
                }),
                range: None,
            }));
        }

        if trimmed.starts_with("!use") {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "**!use** - Activate an existing schema\n\n```tqn\n!use SchemaName\n```\n\nSwitch to a previously defined schema for subsequent rows.".to_string(),
                }),
                range: None,
            }));
        }

        if trimmed.starts_with("!schemas") {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "**!schemas** - Begin schema definition block\n\n```tqn\n!schemas\nUser id name email\nProduct sku price\n---\n```\n\nDefine multiple schemas upfront. Block ends with `---`.".to_string(),
                }),
                range: None,
            }));
        }

        if trimmed.starts_with("!import") {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "**!import** - Include another Tauq file\n\n```tqn\n!import \"path/to/file.tqn\"\n```\n\nImport and merge content from another file.".to_string(),
                }),
                range: None,
            }));
        }

        // Check if hovering over a schema name
        for schema in &doc.schemas {
            if trimmed.contains(&schema.name) {
                let fields_str = schema.fields.join(", ");
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "**Schema: {}**\n\nFields: `{}`\n\nDefined at line {}",
                            schema.name,
                            fields_str,
                            schema.line + 1
                        ),
                    }),
                    range: None,
                }));
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let docs = self.documents.read().await;
        let doc = docs.get(uri);

        let lines: Vec<&str> = doc.map(|d| d.content.lines().collect()).unwrap_or_default();

        let line_idx = position.line as usize;
        let current_line = lines.get(line_idx).unwrap_or(&"");
        let char_idx = position.character as usize;
        let prefix = if char_idx <= current_line.len() {
            &current_line[..char_idx]
        } else {
            current_line
        };

        let mut items = Vec::new();

        // Complete directives after !
        if prefix.trim().ends_with('!') || prefix.trim().starts_with('!') {
            items.extend(vec![
                CompletionItem {
                    label: "!def".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Define and activate a schema".to_string()),
                    insert_text: Some("def ".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "!use".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Activate an existing schema".to_string()),
                    insert_text: Some("use ".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "!schemas".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Begin schema definition block".to_string()),
                    insert_text: Some("schemas\n".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "!import".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Import another file".to_string()),
                    insert_text: Some("import \"".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "!set".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Set a variable (TQQ)".to_string()),
                    insert_text: Some("set ".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "!emit".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Execute command and insert output (TQQ)".to_string()),
                    insert_text: Some("emit ".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "!pipe".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Pipe remaining content through command (TQQ)".to_string()),
                    insert_text: Some("pipe ".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "!run".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Execute code block (TQQ)".to_string()),
                    insert_text: Some("run python3 {\n\n}".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "!json".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Convert JSON file to Tauq inline (TQQ)".to_string()),
                    insert_text: Some("json \"".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "!read".to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Read file contents as string (TQQ)".to_string()),
                    insert_text: Some("read \"".to_string()),
                    ..Default::default()
                },
            ]);
        }

        // Complete schema names after !use
        if prefix.trim().starts_with("!use ")
            && let Some(doc) = doc
        {
            for schema in &doc.schemas {
                items.push(CompletionItem {
                    label: schema.name.clone(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some(format!("Schema with {} fields", schema.fields.len())),
                    ..Default::default()
                });
            }
        }

        // Complete constants
        items.extend(vec![
            CompletionItem {
                label: "true".to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Boolean true".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "false".to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Boolean false".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "null".to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Null value".to_string()),
                ..Default::default()
            },
        ]);

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let lines: Vec<&str> = doc.content.lines().collect();
        let line_idx = position.line as usize;

        if line_idx >= lines.len() {
            return Ok(None);
        }

        let line = lines[line_idx];
        let trimmed = line.trim();

        // Go to definition for !use SchemaName
        if let Some(schema_name) = trimmed.strip_prefix("!use ") {
            let schema_name = schema_name.trim();

            for schema in &doc.schemas {
                if schema.name == schema_name {
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: schema.line,
                                character: schema.character,
                            },
                            end: Position {
                                line: schema.line,
                                character: schema.character + 4 + schema.name.len() as u32,
                            },
                        },
                    })));
                }
            }
        }

        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;

        let docs = self.documents.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        // Parse and reformat
        let mut parser = Parser::new(&doc.content);
        match parser.parse() {
            Ok(json_val) => {
                let formatted = tauq::json_to_tauq(&json_val);

                // Calculate range of entire document
                let lines: Vec<&str> = doc.content.lines().collect();
                let last_line = lines.len().saturating_sub(1);
                let last_char = lines.last().map(|l| l.len()).unwrap_or(0);

                Ok(Some(vec![TextEdit {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: last_line as u32,
                            character: last_char as u32,
                        },
                    },
                    new_text: formatted,
                }]))
            }
            Err(_) => Ok(None), // Don't format if there are parse errors
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;

        let docs = self.documents.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let mut tokens: Vec<SemanticToken> = Vec::new();
        let mut prev_line = 0u32;
        let mut prev_char = 0u32;

        for (line_num, line) in doc.content.lines().enumerate() {
            let line_num = line_num as u32;

            // Highlight comments
            if let Some(idx) = line.find('#') {
                let delta_line = line_num - prev_line;
                let delta_start = if delta_line == 0 {
                    idx as u32 - prev_char
                } else {
                    idx as u32
                };

                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: (line.len() - idx) as u32,
                    token_type: 5, // COMMENT
                    token_modifiers_bitset: 0,
                });

                prev_line = line_num;
                prev_char = idx as u32;
            }

            // Highlight directives
            let trimmed = line.trim();
            if trimmed.starts_with('!') {
                let start_char = line.find('!').unwrap_or(0) as u32;
                let directive_end = trimmed.find(' ').unwrap_or(trimmed.len());

                let delta_line = line_num - prev_line;
                let delta_start = if delta_line == 0 {
                    start_char - prev_char
                } else {
                    start_char
                };

                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: directive_end as u32,
                    token_type: 0, // KEYWORD
                    token_modifiers_bitset: 0,
                });

                prev_line = line_num;
                prev_char = start_char;
            }
        }

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
