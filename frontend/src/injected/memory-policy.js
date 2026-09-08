(function () {
  if (window.__dshDockMemoryPolicyApplied) return;
  window.__dshDockMemoryPolicyApplied = true;
  // 能力探测：不支持的引擎（老 WebKitGTK 等）直接退出，零副作用。
  if (!(window.CSS && CSS.supports && CSS.supports('content-visibility', 'auto'))) return;
  var ROW = '[data-chat-anchor-key]';
  var FLOW = '[data-chat-flow]';
  var STREAMING = '[data-streaming]';
  var SKIP = 'dsh-cv-skip';

  // 注入 CSS：一条规则覆盖全部行（含未来插入的），豁免类后置覆盖；
  // 补齐列表内边距，防止 content-visibility 隐式 Paint Containment 裁切序号。
  var style = document.createElement('style');
  style.id = 'dsh-dock-memory-policy';
  style.textContent =
    FLOW + ' > ' + ROW + ' { content-visibility: auto; contain-intrinsic-size: auto 64px; }' +
    FLOW + ' > ' + ROW + '.' + SKIP + ' { content-visibility: visible; }' +
    FLOW + ' ol, ' + FLOW + ' ul { padding-left: 1.5em !important; }';
  (document.head || document.documentElement).appendChild(style);

  // 活豁免：流式行加 SKIP 类，流式结束移除——内容增长期保持完整渲染。
  function syncStreaming(root) {
    if (!root || !root.querySelectorAll) return;
    var rows = (root.matches && root.matches(FLOW))
      ? root.querySelectorAll(':scope > ' + ROW)
      : root.querySelectorAll(FLOW + ' > ' + ROW);
    for (var i = 0; i < rows.length; i++) {
      var row = rows[i];
      var streaming = row.querySelector(STREAMING) !== null;
      if (streaming && !row.classList.contains(SKIP)) row.classList.add(SKIP);
      else if (!streaming && row.classList.contains(SKIP)) row.classList.remove(SKIP);
    }
  }

  // document-start 时 body 尚不存在（WKUserScript 时序）：先观察
  // documentElement；body 就绪后再补全量同步。
  var mo = new MutationObserver(function (muts) {
    for (var m = 0; m < muts.length; m++) {
      var mut = muts[m];
      if (mut.type === 'attributes') {
        // data-streaming 增删：同步该行所在 flow 的豁免类。
        if (mut.target && mut.target.closest && mut.target.closest(FLOW) !== null) {
          syncStreaming(mut.target.closest(FLOW));
        }
        continue;
      }
      if (mut.type !== 'childList') continue;
      if (mut.target && mut.target.closest && mut.target.closest(FLOW) !== null) {
        syncStreaming(mut.target.closest(FLOW));
      }
    }
  });
  mo.observe(document.documentElement, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['data-streaming'],
  });
  if (document.body) syncStreaming(document);
  else document.addEventListener('DOMContentLoaded', function () { syncStreaming(document); });
})();
