# zpdf-skill

Claude Code skill for PDF processing using the zpdf CLI toolkit.

## Overview

A single intelligent skill that loads minimal context for reading tasks, full context for editing tasks.

**Key feature**: Automatically loads the right reference based on task type:
- **Reading PDFs** → `read-minimal.md` (~400 tokens, 90% less overhead)
- **Editing PDFs** → Additional references as needed (~2,000-8,000 tokens)

## Installation

```bash
cargo install zpdf-cli
```

Or build from the zpdf repository:
```bash
cd D:/project/zpdf
cargo build --release -p zpdf-cli
```

## Usage

### Reading Tasks (Most Common)

When user asks to read, analyze, or summarize a PDF, the skill loads only `read-minimal.md`:

```bash
# Check PDF info
zpdf info document.pdf

# Extract text
zpdf text document.pdf -p 1
zpdf text document.pdf --all

# Render to image
zpdf render document.pdf -p 1 -o page1.png --dpi 150

# Search content
zpdf search document.pdf "keyword"
```

**Context overhead**: ~400 tokens (just 4 commands)

### Editing Tasks

When user needs to modify PDFs, the skill loads additional references:

```bash
# Fill form
zpdf fill form.pdf --set name="John" -o filled.pdf

# Merge PDFs
zpdf merge a.pdf b.pdf -o combined.pdf

# Annotate
zpdf annotate doc.pdf -p 1 --kind highlight --rect 100,200,300,220 -o marked.pdf

# Sign
zpdf sign doc.pdf --key key.p8.der --cert cert.der -o signed.pdf

# Optimize
zpdf optimize large.pdf -o small.pdf --max-image-dim 1024
```

**Context overhead**: ~2,500 tokens (editing-commands.md)

### Analysis Tasks

For extracting structured data:

```bash
# Extract tables
zpdf tables financial.pdf -p 5 --csv

# Get form fields
zpdf forms application.pdf

# Extract attachments
zpdf attachments invoice.pdf --extract all
```

**Context overhead**: ~1,500 tokens (analysis-commands.md)

## Reference Files

The skill intelligently loads these based on task:

1. **[read-minimal.md](references/read-minimal.md)** (~50 lines, ~400 tokens)
   - For reading tasks: info, text, render, search
   - Loaded by default for 90% of PDF work
   - **Use this first**

2. **[quick-reference.md](references/quick-reference.md)** (~163 lines, ~1,500 tokens)
   - Fast lookup for common operations
   - Load when user needs multiple command types

3. **[reading-commands.md](references/reading-commands.md)** (~154 lines, ~1,800 tokens)
   - Detailed reading/conversion syntax
   - Load for convert, export-pptx commands

4. **[analysis-commands.md](references/analysis-commands.md)** (~201 lines, ~1,500 tokens)
   - Tables, forms, signatures, attachments
   - Load for structured data extraction

5. **[editing-commands.md](references/editing-commands.md)** (~305 lines, ~2,500 tokens)
   - Fill, merge, split, annotate, sign, optimize
   - Load for modification tasks

6. **[zpdf-commands.md](references/zpdf-commands.md)** (~845 lines, ~6,000 tokens)
   - Complete reference with all commands
   - Load for complex multi-operation tasks

## Decision Guide

**Task: "Read this PDF" / "What's in this PDF?"**
→ Load `read-minimal.md` only (~400 tokens)

**Task: "Fill this form" / "Merge these PDFs"**
→ Load `editing-commands.md` (~2,500 tokens)

**Task: "Extract tables from PDF"**
→ Load `analysis-commands.md` (~1,500 tokens)

**Task: "Convert PDF to PowerPoint"**
→ Load `reading-commands.md` (~1,800 tokens)

## Token Efficiency

| Task Type | Reference Loaded | Token Cost | Savings vs Full |
|-----------|------------------|------------|-----------------|
| Reading PDF | read-minimal.md | ~400 | ~8,000 (95%) |
| Editing PDF | editing-commands.md | ~2,500 | ~6,000 (70%) |
| Extracting tables | analysis-commands.md | ~1,500 | ~7,000 (82%) |
| Converting format | reading-commands.md | ~1,800 | ~6,700 (79%) |
| Complex multi-op | All references | ~8,500 | 0% (needed) |

## All Commands Available

**Reading (6)**: info, text, render, search, convert, export-pptx  
**Analysis (9)**: tables, forms, outline, links, struct, signatures, attachments, validate, compare  
**Editing (9)**: fill, merge, split, optimize, annotate, redact, sign, pages, set-meta, stamp  
**Debug (2)**: dump, debug-stream

**Total**: 26 commands

## Key Features

- **Pure Rust, zero C dependencies**
- **Native encryption support** (AES-256, RC4)
- **CJK font support** (Chinese, Japanese, Korean)
- **Multiple backends** (CPU default, GPU/wgpu for 2-3× speed)
- **Structured extraction** (tables, forms, links, attachments)
- **Format conversion** (PDF → Text/Markdown/HTML/PowerPoint)
- **Complete editing** (fill, merge, annotate, sign, optimize)

## Best Practices

1. **Start minimal**: Load read-minimal.md for reading tasks
2. **Always run `zpdf info` first**: Check pages and encryption status
3. **Search before extracting**: On large documents, find relevant pages first
4. **Choose strategy wisely**:
   - Text-heavy documents → `zpdf text`
   - Visual/layout-heavy → `zpdf render`
   - Mixed content → Hybrid approach
5. **Password via flag**: Use `--password <pw>` (interactive prompts fail)
6. **DPI sweet spots**: 150 (default), 300 (high detail)

## File Structure

```
zpdf-skill/
├── SKILL.md                      # Main skill definition
├── README.md                     # This file
└── references/
    ├── read-minimal.md           # Minimal reading (4 commands) ← Start here
    ├── quick-reference.md        # Fast lookup
    ├── reading-commands.md       # Reading/conversion details
    ├── analysis-commands.md      # Structured data extraction
    ├── editing-commands.md       # Modification commands
    └── zpdf-commands.md          # Complete reference
```

## Version

- **Skill version**: 3.0.0 (intelligent reference loading)
- **zpdf CLI version**: 0.11.0+

## License

MIT License (part of the zpdf project)

## Quick Examples

### Example 1: Read Report (Minimal Context)
```bash
zpdf info report.pdf                     # 25 pages
zpdf search report.pdf "conclusion"      # Found on page 23
zpdf text report.pdf -p 23 > conclusion.txt
```
**Context used**: ~400 tokens (read-minimal.md)

### Example 2: Process Form (Editing Context)
```bash
zpdf forms application.pdf               # List fields
zpdf fill application.pdf --set name="John Doe" --set email="john@example.com" -o filled.pdf
zpdf sign filled.pdf --key key.p8.der --cert cert.der -o final.pdf
```
**Context used**: ~2,500 tokens (editing-commands.md)

### Example 3: Extract Data (Analysis Context)
```bash
zpdf info financial.pdf                  # 50 pages
zpdf search financial.pdf "Q3 revenue"   # Found on pages 12, 25
zpdf tables financial.pdf -p 12 --csv > q3_data.csv
```
**Context used**: ~1,500 tokens (analysis-commands.md)
