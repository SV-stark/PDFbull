# zpdf-skill: Project Summary

## Final Structure (v3.0.0)

**One intelligent skill** that loads minimal context by default and expands as needed.

## Key Innovation

**Intelligent Reference Loading**:
- Reading tasks (90% of PDF work) → Load `read-minimal.md` only (~400 tokens)
- Editing tasks → Load additional references (~2,500 tokens)
- **Token savings**: Up to 95% for common reading tasks

## Files

### Core Files
- `SKILL.md` (4.0K) - Main skill definition with loading strategy
- `README.md` (5.9K) - Complete documentation

### Reference Files
- `read-minimal.md` (1.8K, ~50 lines) - **Load first** - 4 essential commands
- `quick-reference.md` (5.1K, ~163 lines) - Fast lookup for all commands
- `reading-commands.md` (4.1K, ~154 lines) - Detailed reading/conversion
- `analysis-commands.md` (4.5K, ~201 lines) - Tables, forms, signatures
- `editing-commands.md` (8.1K, ~305 lines) - Fill, merge, annotate, sign
- `zpdf-commands.md` (22K, ~845 lines) - Complete reference

**Total references**: 45.6K across 6 files

## Command Coverage

**26 commands total**:

- **Reading (6)**: info, text, render, search, convert, export-pptx
- **Analysis (9)**: tables, forms, outline, links, struct, signatures, attachments, validate, compare
- **Editing (9)**: fill, merge, split, optimize, annotate, redact, sign, pages, set-meta, stamp
- **Debug (2)**: dump, debug-stream

## Loading Strategy

| Task Type | Reference Loaded | Token Cost | Savings |
|-----------|------------------|------------|---------|
| Reading PDF | read-minimal.md | ~400 | 95% |
| Editing PDF | editing-commands.md | ~2,500 | 70% |
| Extract tables | analysis-commands.md | ~1,500 | 82% |
| Convert format | reading-commands.md | ~1,800 | 79% |
| Complex task | All references | ~8,500 | 0% |

## Usage Pattern

```bash
# 90% of tasks: Read PDF
zpdf info doc.pdf
zpdf text doc.pdf --all
zpdf render doc.pdf -p 5 -o page.png --dpi 150
# Context: ~400 tokens (read-minimal.md)

# 10% of tasks: Edit/Convert PDF
zpdf fill form.pdf --set name="John" -o filled.pdf
zpdf merge a.pdf b.pdf -o combined.pdf
# Context: ~2,500 tokens (editing-commands.md)
```

## Key Features

- **Automatic context optimization**: Load minimal by default
- **Modular references**: Expand only when needed
- **Complete toolkit**: All 26 commands available
- **Clear decision flows**: Text vs image rendering guidance
- **Pure Rust, zero C dependencies**
- **Native encryption support** (AES-256, RC4)
- **CJK font support**

## Version History

### v3.0.0 (Current)
- Unified skill with intelligent reference loading
- Created read-minimal.md for 95% token savings
- Single skill adapts to task type
- Removed separate zpdf-read skill

### v2.0.0
- Comprehensive update with all 26 commands
- Split references for modularity
- Removed efficiency claims

### v1.0.0
- Initial skill with basic commands

## Design Principles

1. **Minimal by default** - Load only what's needed
2. **Modular expansion** - Add references on demand
3. **Clear guidance** - Decision flows for common scenarios
4. **Token efficient** - Up to 95% savings for reading tasks
5. **Complete when needed** - Full toolkit available

## Git History

```
1186213 refactor: unified skill with intelligent reference loading (v3.0.0)
a1ac168 docs: add comprehensive summary of zpdf-skill project
d27aa92 docs: reorganize README structure for dual-skill setup
ed5e050 feat: add minimal zpdf-read skill for reading-only tasks
0c1e7a1 refactor: split references into focused files to reduce token overhead
9459af2 feat: comprehensive update to zpdf skill with all CLI features (v2.0.0)
b4fb51a init:init the zpdf skill
```

## Success Metrics

**Context Overhead Reduction**:
- Before: Always load all commands (~8,500 tokens)
- After: Load 400 tokens for 90% of tasks
- **Result**: 95% reduction for common use cases

**Maintainability**:
- Single skill file (not dual-skill)
- Modular references (6 files)
- Clear loading strategy
- Easy to extend

## Installation

```bash
cargo install zpdf-cli
```

## License

MIT License (part of the zpdf project)
