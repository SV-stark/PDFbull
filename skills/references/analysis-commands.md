# Analysis Commands Reference

Commands for extracting structured data and analyzing PDF content.

## tables — Extract Tables
```bash
zpdf tables <file.pdf> [-p <page>] [--all] [--csv] [--password <pw>]
```

**Flags**:
- `-p <N>` — Extract tables from single page
- `--all` — Extract from all pages
- `--csv` — Output in CSV format
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
zpdf tables financial.pdf -p 5
zpdf tables financial.pdf -p 5 --csv > data.csv
zpdf tables report.pdf --all --csv > all_tables.csv
```

**Output**: Structured table data (text or CSV format).

---

## forms — Extract Form Fields
```bash
zpdf forms <file.pdf> [--password <pw>]
```

**Output**: List of all interactive form fields with current values.

**Example**:
```bash
$ zpdf forms application.pdf
Field: name (Type: text) = "John Doe"
Field: email (Type: text) = "john@example.com"
Field: agree (Type: checkbox) = checked
Field: country (Type: dropdown) = "United States"
```

**Use when**: Reading filled forms or understanding form structure before filling.

---

## outline — Document Outline
```bash
zpdf outline <file.pdf> [--password <pw>]
```

**Output**: Hierarchical table of contents with page references.

**Example**:
```bash
$ zpdf outline manual.pdf
1. Introduction (page 1)
2. Getting Started (page 5)
   2.1. Installation (page 6)
   2.2. Configuration (page 10)
3. Advanced Topics (page 25)
```

**Use when**: Understanding document structure before deep reading.

---

## links — Extract Hyperlinks
```bash
zpdf links <file.pdf> [--password <pw>]
```

**Output**: All hyperlinks in the document.

**Example**:
```bash
$ zpdf links doc.pdf
Page 3: https://example.com/documentation
Page 3: Page 15 (internal link)
Page 5: mailto:support@example.com
```

**Use when**: Extracting references, checking external resources.

---

## struct — PDF Structure Tree
```bash
zpdf struct <file.pdf> [--password <pw>]
```

**Output**: Accessibility structure (tagged PDF markup).

**Use when**: Understanding document semantics, accessibility analysis.

---

## signatures — Verify Digital Signatures
```bash
zpdf signatures <file.pdf> [--trust <roots.pem>] [--password <pw>]
```

**Flags**:
- `--trust <path>` — PEM file with trusted root certificates
- `--password <pw>` — Decrypt with password

**Example**:
```bash
$ zpdf signatures signed.pdf
Signature 1: Valid
  Signer: John Doe (john@example.com)
  Timestamp: 2024-07-26 10:30:45 UTC
  Certificate: CN=John Doe, O=Example Corp
  Valid: Yes
```

**Use when**: Verifying document authenticity.

---

## attachments — Embedded Files
```bash
zpdf attachments <file.pdf> [--extract <index|name|all>] [--out-dir <dir>] [--password <pw>]
```

**Flags**:
- `--extract <spec>` — Extract by index, name, or all
- `--out-dir <dir>` — Output directory (default: current dir)
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
zpdf attachments invoice.pdf
zpdf attachments invoice.pdf --extract all --out-dir ./extracted/
```

**Use when**: Finding or extracting embedded documents (PDF portfolios, ZUGFeRD invoices).

---

## validate — PDF/A & PDF/UA Validation
```bash
zpdf validate <file.pdf> [--profile pdfa-1b|pdfa-2b|pdfua-1] [--password <pw>]
```

**Flags**:
- `--profile <spec>` — Conformance profile to validate against:
  - `pdfa-1b` / `pdfa-2b` — PDF/A archival conformance (encryption, `/ID`, header version, XMP `pdfaid`, `GTS_PDFA1` output intent + ICC, font embedding, forbidden features incl. JS/Launch actions, embedded files, transparency, forbidden annotation subtypes)
  - `pdfua-1` — PDF/UA-1 accessibility conformance (tagged, structure tree, `/Lang`, figure alt-text, headings, role mapping, table structure, annotation `/OBJR` coverage, `/StructParents`)
- `--password <pw>` — Decrypt with password

**Output**: Validation result with any violations (exits with code 3 on FAIL for PDF/A).

**Examples**:
```bash
# PDF/A-1b archival check
zpdf validate archive.pdf --profile pdfa-1b

# PDF/UA-1 accessibility check (after `zpdf tag`)
zpdf validate doc.pdf --profile pdfua-1
```

**Use when**: Checking PDF/A compliance for archival, or PDF/UA compliance for accessibility (e.g. after `zpdf tag` adds a structure tree).

---

## compare — Visual Diff
```bash
zpdf compare <a.png> <b.png> [--out <diff.png>] [--threshold <0-255>]
```

**Flags**:
- `--out <path>` — Output difference image
- `--threshold <N>` — Color difference threshold (default: 10)

**Output**: Visual differences between two rendered pages.

**Use when**: Tracking document changes, regression testing.

---

## Common Patterns

### Extract All Structured Data
```bash
zpdf outline doc.pdf > structure.txt
zpdf links doc.pdf > links.txt
zpdf forms doc.pdf > fields.txt
zpdf attachments doc.pdf --extract all --out-dir ./files/
```

### Table Extraction Pipeline
```bash
# Find pages with tables
zpdf search report.pdf "Table" | grep "Page" | awk '{print $2}' | sort -u > pages.txt

# Extract tables from those pages
for p in $(cat pages.txt); do
  echo "=== Page $p ===" >> tables.csv
  zpdf tables report.pdf -p $p --csv >> tables.csv
done
```

### Verify Signed Documents
```bash
zpdf signatures contract.pdf --trust trusted_roots.pem > verification.txt
if grep -q "Valid: Yes" verification.txt; then
  echo "Signature verified"
else
  echo "Signature invalid"
fi
```
