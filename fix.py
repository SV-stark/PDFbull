
import os
import re

def repl(p, pattern, repl, count=0, flags=0):
    try:
        with open(p, 'r', encoding='utf-8') as f:
            c = f.read()
        c2 = re.sub(pattern, repl, c, count=count, flags=flags)
        with open(p, 'w', encoding='utf-8') as f:
            f.write(c2)
    except Exception as e:
        print(f'Error processing {p}: {e}')

# pdf_engine.rs
p = 'src/pdf_engine.rs'
repl(p, r'const PAGE_RENDER_EXTRAS: f32 = 1.5;
', '')
repl(p, r'const THUMB_BUFFER_BEHIND: usize = 15;
', '')
repl(p, r'const THUMB_BUFFER_AHEAD: usize = 45;
', '')
repl(p, r'if !current_word.is_empty\(\) \{\s+if let Some\(rect\) = word_rect \{', 'if !current_word.is_empty()\n                    && let Some(rect) = word_rect {')
# also handle closing brace for the first if
# Actually, wait, replacing just the check will leave an unbalanced brace.
