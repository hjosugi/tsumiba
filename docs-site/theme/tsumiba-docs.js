// Adds two links to the menu bar of each book: the site's front page, and the
// same page in the other language. Both books have identical page names, so the
// counterpart is a path swap. No network requests; nothing is stored.
(function () {
  'use strict';

  function ready(fn) {
    if (document.readyState !== 'loading') fn();
    else document.addEventListener('DOMContentLoaded', fn);
  }

  ready(function () {
    var bar = document.querySelector('.right-buttons');
    if (!bar) return;

    // mdBook defines `path_to_root` ("" at the book's top level).
    var root = typeof path_to_root === 'string' ? path_to_root : '';
    var ja = document.documentElement.lang === 'ja';
    var page = location.pathname.replace(/^.*\//, '') || 'index.html';

    function add(href, text, title, className) {
      var a = document.createElement('a');
      a.href = href;
      a.textContent = text;
      a.title = title;
      a.setAttribute('aria-label', title);
      a.className = 'tsu-nav ' + className;
      bar.insertBefore(a, bar.firstChild);
    }

    // Inserted last-to-first, so the order on screen is: home, language.
    add(
      root + '../' + (ja ? 'en' : 'ja') + '/' + page + location.hash,
      ja ? 'English' : '日本語',
      ja ? 'Read this page in English' : 'このページを日本語で読む',
      'tsu-lang'
    );
    add(root + '../', ja ? 'ホーム' : 'Home', ja ? 'tsumiba のトップページ' : 'tsumiba front page', 'tsu-home');
  });
})();
