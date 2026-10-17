// The front page works without JavaScript (English, every section visible, install tabs are
// CSS only). This adds: the language switch, and copy buttons. Nothing is fetched or sent.
(function () {
  'use strict';
  var root = document.documentElement;
  var TITLES = {
    en: 'tsumiba — notes and files, one gesture away',
    ja: 'tsumiba — メモとファイルを、画面の端にひと積み'
  };
  var DESCRIPTIONS = {
    en: 'tsumiba is a memo and file shelf that slides in from the top edge of your screen. Windows, macOS and Linux. Open source.',
    ja: 'tsumiba は、画面の上端からスライドして現れる、メモとファイルの置き場。Windows・macOS・Linux 対応のオープンソース。'
  };

  function setLang(lang, remember) {
    if (lang !== 'ja') lang = 'en';
    root.lang = lang;
    document.title = TITLES[lang];
    var meta = document.querySelector('meta[name="description"]');
    if (meta) meta.setAttribute('content', DESCRIPTIONS[lang]);
    document.querySelectorAll('.langs button').forEach(function (b) {
      b.setAttribute('aria-pressed', String(b.getAttribute('data-set-lang') === lang));
    });
    if (remember) {
      try { localStorage.setItem('tsumiba-lang', lang); } catch (e) { /* private mode */ }
    }
  }

  document.querySelectorAll('.langs button').forEach(function (b) {
    b.addEventListener('click', function () { setLang(b.getAttribute('data-set-lang'), true); });
  });
  setLang(root.lang, false);

  document.querySelectorAll('.copy').forEach(function (button) {
    button.addEventListener('click', function () {
      var code = button.parentElement.querySelector('code');
      if (!code || !navigator.clipboard) return;
      var text = code.textContent.replace(/^\$ /gm, '');
      navigator.clipboard.writeText(text).then(function () {
        var before = button.textContent;
        button.textContent = root.lang === 'ja' ? 'コピーしました' : 'Copied';
        setTimeout(function () { button.textContent = before; }, 1600);
      });
    });
  });
})();
