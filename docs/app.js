// PDFbull GitHub Pages Interactive Logic

document.addEventListener('DOMContentLoaded', () => {
  // 1. Copy Terminal Command
  const copyBtn = document.getElementById('copyCmdBtn');
  if (copyBtn) {
    copyBtn.addEventListener('click', () => {
      const copyText = copyBtn.getAttribute('data-copy') || 'cargo install pdfbull';
      navigator.clipboard.writeText(copyText).then(() => {
        const textSpan = copyBtn.querySelector('.copy-text');
        if (textSpan) {
          const original = textSpan.textContent;
          textSpan.textContent = 'Copied!';
          copyBtn.style.borderColor = 'var(--accent-emerald)';
          copyBtn.style.color = 'var(--accent-emerald)';

          setTimeout(() => {
            textSpan.textContent = original;
            copyBtn.style.borderColor = '';
            copyBtn.style.color = '';
          }, 2000);
        }
      });
    });
  }

  // 2. OCR Script Switcher Demo
  const ocrTabs = document.querySelectorAll('.ocr-tab-btn');
  const ocrSampleText = document.getElementById('ocrSampleText');
  const recModelVal = document.getElementById('recModelVal');

  const ocrData = {
    devanagari: {
      model: 'devanagari_PP-OCRv4_rec.rten',
      words: ['नमस्ते', 'संसार!', 'PDFbull', 'ऑप्टिकल', 'कैरेक्टर', 'रिकग्निशन', 'इंजन।']
    },
    latin: {
      model: 'text-recognition.rten',
      words: ['PDFbull', 'Pure-Rust', 'OCR', 'Engine', 'Sub-45ms', 'Offline', 'Inference']
    }
  };

  ocrTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      ocrTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      const scriptKey = tab.getAttribute('data-script');
      const data = ocrData[scriptKey];

      if (data && ocrSampleText && recModelVal) {
        recModelVal.innerHTML = `<code>${data.model}</code>`;
        ocrSampleText.innerHTML = data.words
          .map(w => `<span class="ocr-word-box">${w}</span>`)
          .join(' ');
      }
    });
  });
});
