/* The library workflow kit: the frame without the page switch, the library
   pane at the bottom, device markers, the audition bar, drag and drop. Built on
   the redesign's kit (TP) and loaded after it. Every function returns HTML. */
(function () {
  const { I, G, z } = TP;
  const TL = {};
  TL.size = () => TP.size();

  /* ---------- device marker ---------- */
  TL.dm = (dev, o = {}) => {
    const cls = ['dm'];
    if (o.no) cls.push('no');
    if (o.only) cls.push('only');
    if (o.lg) cls.push('lg');
    return `<span class="${cls.join(' ')}"><span class="f">${dev.f}</span>${o.only ? '' : `<span class="m">${dev.m}</span>`}</span>`;
  };
  // Which tones the connected pedal plays. An HX Stomp XL also plays HX Stomp
  // tones; a PRO on 2.x plays 1.5 tones (with 2.x's default chain).
  TL.plays = (connected, dev) => {
    if (!connected) return false;
    if (connected === 'stomp') return dev.f === 'HX' && dev.m === 'Stomp';
    if (connected === 'xl') return dev.f === 'HX' && (dev.m === 'Stomp' || dev.m === 'Stomp XL');
    if (connected === 'pro2') return dev.f === 'PRO';
    return false;
  };

  /* ---------- sidebar: the pedal, its presets, its protection ---------- */
  TL.prow = function (p) {
    const cls = ['prow'];
    if (p.bank) cls.push('bank');
    if (p.sel) cls.push('sel');
    if (p.empty) cls.push('empty');
    if (p.hov) cls.push('hov');
    if (p.target) cls.push('target', p.empty ? 'free' : 'used');
    if (p.drop) cls.push('drop');
    if (p.can) cls.push('can');
    if (p.lock) cls.push('lock');
    if (p.dragsrc) cls.push('dragsrc');
    let marks = '';
    if (p.aud) marks += `<span class="spk" title="its edit buffer is playing an audition">${I('volume-2', 's13')}</span>`;
    if (p.dirty) marks += `<span class="edited" title="edited"></span>`;
    if (p.fav && !p.target) marks += `<span style="color:var(--accent);display:flex">${I('star', 's12')}</span>`;
    if (p.lib === 'same' && !p.target) marks += `<span style="display:flex;color:var(--faint)">${I('check', 's13')}</span>`;
    if (p.lib === 'differs' && !p.target) marks += `<span style="display:flex;color:var(--hot)">${I('git-compare', 's13')}</span>`;
    if (p.lockIcon) marks += `<span style="display:flex;color:var(--faint)">${I('lock', 's12')}</span>`;
    if (p.hint) marks += `<span class="hint">${p.hint}</span>`;
    if (p.flash) cls.push('flash');
    const name = p.empty ? (p.target ? 'Empty' : 'New Preset') : p.name;
    return `<div class="${cls.join(' ')}" ${p.id ? `id="${p.id}"` : ''}><span class="sl num">${p.slot}</span><span class="nm">${name}</span><span class="mk">${marks}</span></div>`;
  };
  TL.sidebar = function (o) {
    const dev = o.device;
    let h = `<aside class="side ${o.dropAll ? 'droptarget' : ''}">`;
    h += `<div class="devcard ${dev.online ? '' : 'off'} ${o.devSel ? 'sel' : ''}"><div class="well">${I(dev.icon || 'tp-pedal', 's18')}</div>
      <div class="grow"><div class="nm ell">${dev.name}</div><div class="st"><span class="dot"></span><span class="ell">${dev.status}</span></div></div>
      <span class="chev">${I('chevrons-up-down', 's14')}</span></div>`;
    if (o.picking) h += `<div class="picking">${o.picking}</div>`;
    h += `<div class="sec"><span class="t">${o.listTitle || 'Presets'}</span><span class="aside">${o.listAside || ''}</span>
      <span class="iconbtn sm">${I('star', 's14')}</span><span class="iconbtn sm">${I('tp-computer', 's14')}</span></div>`;
    h += `<div class="plist">${(o.presets || []).map(TL.prow).join('')}<div class="fade"></div></div>`;
    if (o.foot !== false) h += `<div class="sfoot">${o.foot || ''}</div>`;
    return h + `</aside>`;
  };
  TL.hxSide = function (o = {}) {
    const presets = D.hxPresets().slice(0, o.count || z(16, 20, 46));
    presets[1].sel = !o.noSel; presets[1].dirty = o.dirty !== false;
    if (o.aud) presets[1].aud = true;
    if (o.tweak) o.tweak(presets);
    return TL.sidebar(Object.assign({
      device: { name: 'HX Stomp', status: 'Connected · 3.80', online: true },
      listAside: '42 of 126', presets,
      foot: `<span class="ok row">${I('shield-check', 's14')}</span><span class="grow ell">Backed up at 14:02</span><span class="iconbtn sm">${I('settings', 's14')}</span>`,
    }, o.side || {}));
  };

  /* ---------- the library pane ---------- */
  TL.tabs = function (o) {
    const t = o.tab || 'tones', st = o.tabState || {};
    const tab = (k, label, n, icon) => {
      const cls = ['ltab'];
      if (t === k && !st[k]) cls.push('on');
      if (st[k]) cls.push(st[k]);
      const drop = st[k] === 'drop' && o.dropLabel && o.dropLabel[k];
      const lab = drop ? `${I(o.dropIcon && o.dropIcon[k] || 'arrow-down-to-line')}${o.dropLabel[k]}` : `${icon ? I(icon) : ''}${label}${n != null ? ` <span class="n num">${n}</span>` : ''}`;
      return `<span class="${cls.join(' ')}">${lab}</span>`;
    };
    const c = o.counts || {};
    return `<div class="ltabs">${tab('tones', 'Tones', c.tones != null ? c.tones : 58)}${tab('setlists', 'Setlists', c.setlists != null ? c.setlists : 5)}${tab('cloud', 'Cloud', c.cloud, 'cloud')}</div>`;
  };
  TL.paneHead = function (o) {
    const s = TL.size();
    let left = TL.tabs(o);
    if (o.tab === 'cloud' && !o.closed) {
      left += `<span class="vsep" style="margin:0 2px"></span><div class="seg" style="height:28px"><span class="s ${o.scope !== 'mine' ? 'on' : ''}" style="height:22px">${I('users', 's13')}Everyone</span><span class="s ${o.scope === 'mine' ? 'on' : ''}" style="height:22px">${I('user', 's13')}Mine <span class="n num">${o.mineCount || 8}</span></span></div>`;
    }
    let right = '';
    if (o.closed) {
      right = (o.closedRight || '') + `<span class="iconbtn" title="Show the library (Ctrl L)">${I('panel-bottom-open')}</span>`;
    } else if (o.right != null) {
      right = o.right;
    } else {
      const ph = o.search || (o.tab === 'cloud' ? (o.scope === 'mine' ? 'Search what you published' : 'Search TonePush') : o.tab === 'setlists' ? 'Search setlists' : `Search ${o.counts && o.counts.tones || 58} tones`);
      const sw = o.tab === 'cloud' ? z(140, 168, 260) : z(150, 196, 260);
      right += `<div class="field" style="width:${sw}px;height:28px">${I('search')}<span class="ell">${ph}</span></div>`;
      if (s !== 's') {
        if (o.tab !== 'setlists') right += `<span class="btn sm">${o.filter || (o.tab === 'cloud' && o.scope !== 'mine' ? 'Most downloaded' : 'All tags')}${I('chevron-down', 's12')}</span>`;
        right += `<span class="btn sm">${o.scopeLabel || 'All pedals'}${I('chevron-down', 's12')}</span>`;
        if (o.tab === 'tones' || (o.tab === 'cloud' && s === 'l')) right += `<span class="iconbtn" title="Columns">${I('list')}</span>`;
        right += o.account || (o.tab === 'cloud' && s === 'm' ? `<span class="iconbtn" title="Carmine on TonePush">${I('user')}</span>` : `<span class="btn sm ghost acct">Carmine${I('chevron-down', 's12')}</span>`);
      } else {
        right += `<span class="iconbtn" title="Filter, pedals, columns, account">${I('sliders-horizontal')}</span><span class="iconbtn" title="Details">${I('info')}</span>`;
      }
      right += `<span class="iconbtn" title="Hide the library (Ctrl L)">${I('panel-bottom-close')}</span>`;
    }
    return `<div class="lphead">${left}<span class="grow"></span>${right}</div>`;
  };
  TL.pane = function (o) {
    const h = o.closed ? 41 : o.height;
    return `<section class="lpane ${o.closed ? 'closed' : ''}" style="height:${h + (o.bar ? 46 : 0)}px" id="lpane"><span class="grip"></span>
      ${o.bar || ''}${TL.paneHead(o)}${o.closed ? '' : `<div class="lbody">${o.body || ''}</div>`}${o.extra || ''}</section>`;
  };

  /* ---------- tables ---------- */
  TL.table = function (cols, rows, o = {}) {
    const th = `<div class="th">${cols.map(c => `<span class="c ${c.sorted ? 'sorted' : ''}" style="width:${c.w}px">${c.t || ''}${c.sorted ? ` ${I(c.sorted === 'up' ? 'arrow-up' : 'arrow-down', 's12')}` : ''}</span>`).join('')}</div>`;
    const tr = rows.map(r => `<div class="tr ${r.cls || ''}" ${r.id ? `id="${r.id}"` : ''}>${cols.map(c => `<span class="c ${c.k === 'name' ? 'nm' : ''}" style="width:${c.w}px">${r.cells[c.k] != null ? r.cells[c.k] : ''}</span>`).join('')}</div>`).join('');
    const sb = o.scroll ? `<span class="sbar" style="top:${o.scroll[0]}px;height:${o.scroll[1]}px"></span>` : '';
    return `<div class="ptbl" ${o.id ? `id="${o.id}"` : ''} style="${o.style || ''}">${th}<div style="position:relative;flex:1;min-height:0;overflow:hidden">${tr}</div>${sb}${o.fade === false ? '' : '<div class="fade"></div>'}${o.over || ''}</div>`;
  };
  const songCell = (t) => t.artist ? `${t.song} <span class="muted">· ${t.artist}</span>` : `<span class="muted">${t.song}</span>`;
  TL.placeCell = function (t, st, connected) {
    const plays = TL.plays(connected, t.dev);
    let pedal;
    if (st === 'play') pedal = `<span class="acc" title="playing on the pedal">${I('volume-2', 's14')}</span>`;
    else if (st === 'load') pedal = `<span class="acc">${I('loader-circle', 's14')}</span>`;
    else if (!connected) pedal = `<span title="no pedal">${I('tp-pedal', 's14')}</span>`;
    else if (!plays) pedal = `<span class="no" title="the HX Stomp cannot play it">${I('ban', 's14')}</span>`;
    else if (t.sync === 'same') pedal = `<span class="on" title="on the pedal at ${t.pedal}">${I('tp-pedal', 's14')}</span>`;
    else if (t.sync === 'differs') pedal = `<span class="hot" title="the pedal holds another version">${I('tp-pedal', 's14')}</span>`;
    else pedal = `<span>${I('tp-pedal', 's14')}</span>`;
    const cloud = t.cloud === 'published' ? `<span class="on">${I('cloud', 's14')}</span>` : t.cloud === 'changed' ? `<span class="hot" title="the library has a newer version">${I('cloud-upload', 's14')}</span>` : `<span>${I('cloud', 's14')}</span>`;
    return `<span class="pl">${pedal}${cloud}</span>`;
  };
  TL.toneCols = function (o = {}) {
    const s = TL.size();
    if (s === 's') return [
      { k: 'pl', w: 50 }, { k: 'dev', t: 'Pedal', w: 50 }, { k: 'name', t: 'Name', w: 148, sorted: 'up' }, { k: 'chain', t: 'Chain', w: 118 },
      { k: 'song', t: 'Song · Artist', w: 186 }, { k: 'char', t: 'Character', w: 76 }, { k: 'rating', t: 'Rating', w: 80 },
    ];
    if (s === 'm') return [
      { k: 'pl', w: 52 }, { k: 'dev', t: 'Pedal', w: 96 }, { k: 'name', t: 'Name', w: 148, sorted: 'up' }, { k: 'chain', t: 'Chain', w: 124 },
      { k: 'song', t: 'Song · Artist', w: 138 }, { k: 'char', t: 'Character', w: 76 }, { k: 'rating', t: 'Rating', w: 80 },
    ];
    return [
      { k: 'pl', w: 56 }, { k: 'dev', t: 'Pedal', w: 110 }, { k: 'name', t: 'Name', w: 190, sorted: 'up' }, { k: 'chain', t: 'Chain', w: 190 },
      { k: 'song', t: 'Song · Artist', w: 300 }, { k: 'char', t: 'Character', w: 100 }, { k: 'rating', t: 'Rating', w: 96 },
      { k: 'ver', t: 'Version', w: 80 }, { k: 'added', t: 'Added', w: 90 }, { k: 'mod', t: 'Changed', w: 90 },
    ];
  };
  TL.toneRows = function (tones, o = {}) {
    const connected = o.connected === undefined ? 'stomp' : o.connected;
    return tones.map(t => {
      const st = (o.state || {})[t.name];
      const plays = TL.plays(connected, t.dev);
      const cls = [];
      if (st === 'play' || st === 'load') cls.push('play');
      else if (st) cls.push(st);
      return {
        cls: cls.join(' '), id: 'row-' + t.name.replace(/\W+/g, '-').toLowerCase(),
        cells: {
          pl: TL.placeCell(t, st, connected),
          dev: TL.dm(t.dev, { no: connected && !plays, only: TL.size() === 's' }),
          name: t.name,
          chain: TP.miniChain(t.chain),
          song: songCell(t),
          char: t.char,
          rating: `<span class="stars">${TP.stars(t.rating)}</span>`,
          ver: `<span class="muted">${t.ver}</span>`,
          added: `<span class="muted num">${t.added}</span>`,
          mod: `<span class="muted num">${t.mod || t.added}</span>`,
        },
      };
    });
  };
  TL.cloudCols = function () {
    const s = TL.size();
    if (s === 's') return [{ k: 'pl', w: 50 }, { k: 'dev', t: 'Pedal', w: 50 }, { k: 'name', t: 'Name', w: 150 }, { k: 'chain', t: 'Chain', w: 120 }, { k: 'by', t: 'By', w: 110 }, { k: 'dl', t: 'Downloads', w: 84, sorted: 'down' }];
    if (s === 'm') return [{ k: 'pl', w: 52 }, { k: 'dev', t: 'Pedal', w: 96 }, { k: 'name', t: 'Name', w: 144 }, { k: 'chain', t: 'Chain', w: 124 }, { k: 'song', t: 'Song · Artist', w: 132 }, { k: 'by', t: 'By', w: 96 }, { k: 'dl', t: 'Downloads', w: 100, sorted: 'down' }];
    return [{ k: 'pl', w: 56 }, { k: 'dev', t: 'Pedal', w: 110 }, { k: 'name', t: 'Name', w: 190 }, { k: 'chain', t: 'Chain', w: 190 }, { k: 'song', t: 'Song · Artist', w: 300 }, { k: 'by', t: 'By', w: 150 }, { k: 'dl', t: 'Downloads', w: 110, sorted: 'down' }, { k: 'ver', t: 'Version', w: 90 }, { k: 'upd', t: 'Updated', w: 100 }];
  };
  TL.cloudRows = function (list, o = {}) {
    const connected = o.connected === undefined ? 'stomp' : o.connected;
    return list.map(t => {
      const st = (o.state || {})[t.name];
      const plays = TL.plays(connected, t.dev);
      const pedal = st === 'play' ? `<span class="acc">${I('volume-2', 's14')}</span>` : st === 'load' ? `<span class="acc">${I('loader-circle', 's14')}</span>`
        : !plays ? `<span class="no">${I('ban', 's14')}</span>` : `<span>${I('tp-pedal', 's14')}</span>`;
      const comp = t.lib ? `<span class="on" title="in your library">${I('tp-computer', 's14')}</span>` : `<span>${I('tp-computer', 's14')}</span>`;
      return {
        cls: st === 'play' || st === 'load' ? 'play' : st || '',
        cells: {
          pl: `<span class="pl">${pedal}${comp}</span>`, dev: TL.dm(t.dev, { no: !plays, only: TL.size() === 's' }), name: t.name,
          chain: TP.miniChain(t.chain), song: songCell(t), by: `<span class="soft">${t.by}</span>`,
          dl: `<span class="num">${t.downloads}</span>`, ver: `<span class="muted">${t.ver}</span>`, upd: `<span class="muted num">${t.updated}</span>`,
        },
      };
    });
  };

  /* ---------- inspector ---------- */
  TL.wells = (seq) => `<div class="minitiles">${seq.map((c, i) => {
    const w = i ? '<span class="mtw"></span>' : '';
    if (Array.isArray(c)) return w + `<span class="mtstack">${c.map(x => `<span class="mt cat-${x.replace('!', '')} ${x.endsWith('!') ? 'off' : ''}" style="width:24px;height:24px">${G(x.replace('!', ''))}</span>`).join('')}</span>`;
    return w + `<span class="mt cat-${c.replace('!', '')} ${c.endsWith('!') ? 'off' : ''}">${G(c.replace('!', ''))}</span>`;
  }).join('')}</div>`;
  TL.wr = (icon, cls, text, action) => `<div class="wr"><span class="wi ${cls || ''}">${I(icon, 's14')}</span><span class="wt">${text}</span>${action || ''}</div>`;
  TL.inspector = function (o) {
    const w = o.width || z(280, 300, 400);
    return `<aside class="linsp" style="width:${w}px">${o.html}</aside>`;
  };
  /* A tone in the inspector: name, marker and version, its chain, then where
     it is (the pedal, this computer, TonePush), each with its next action. */
  TL.toneInsp = function (t, o = {}) {
    const meta = [t.ver + (o.versions ? ` of ${o.versions}` : ''), o.meta || (t.char ? `${t.char} · kept ${t.added}` : '')].filter(Boolean).join(' · ');
    const wells = o.wells != null ? o.wells : TL.size() === 'l';
    let h = `<div class="ih"><div class="row" style="gap:8px"><div class="tn grow">${t.name}</div>${o.head || ''}</div>
      <div class="tm">${TL.dm(t.dev, { no: o.no })}<span class="ell">${meta}</span></div>
      ${wells ? `<div style="margin-top:10px">${TL.wells(t.chain)}</div>` : ''}</div>`;
    if (o.where) h += `<div class="is"><div class="where">${o.where}</div></div>`;
    if (o.more) h += o.more;
    return TL.inspector({ html: h, width: o.width });
  };
  TL.facts = (pairs) => `<div class="is"><div class="lbl">Song and tone</div><div class="kv2" style="row-gap:7px">${pairs.map(([k, v]) => `<span class="k">${k}</span><span class="v">${v}</span>`).join('')}</div></div>`;

  /* ---------- the audition bar ---------- */
  TL.abar = function (o) {
    const s = TL.size();
    const keys = s === 's' ? '' : `<span class="keys"><span class="kbdk">↑</span><span class="kbdk">↓</span>${s === 'l' ? 'try the next' : 'next'}</span>`;
    return `<div class="abar">${o.lead || `<span class="chip live">${I(o.loading ? 'loader-circle' : 'volume-2')}${o.loading ? 'Loading' : 'Playing'}</span>`}
      <span class="ab-t">${o.text}</span>${o.keys === false ? '' : keys}${o.actions || ''}</div>`;
  };
  TL.keepBtn = (label, o = {}) => `<span class="splitbtn"><span class="btn primary sm" style="height:30px">${label}${o.kbd === false ? '' : '<span class="kbd">Enter</span>'}</span><span class="btn primary sm" style="height:30px">${I('chevron-down', 's14')}</span></span>`;
  TL.backBtn = (label) => `<span class="btn sm" style="height:30px">${label}<span class="kbd">Esc</span></span>`;

  /* ---------- drag and drop ---------- */
  TL.cursor = (x, y) => `<svg class="cursor" style="left:${x}px;top:${y}px" viewBox="0 0 18 22"><path d="M1.5 1.5 L1.5 17.5 L5.6 13.7 L8.4 20 L11.2 18.8 L8.5 12.6 L14.2 12.6 Z" fill="#ffffff" stroke="#111318" stroke-width="1.3" stroke-linejoin="round"/></svg>`;
  TL.ghost = function (o) {
    const g2 = o.no ? `<div class="g2 no">${I('ban')}${o.outcome}</div>` : `<div class="g2">${I(o.icon || 'arrow-down-to-line')}${o.outcome}</div>`;
    const stack = o.count ? `<span class="stack2"></span><span class="stack1"></span><span class="count num">${o.count}</span>` : '';
    const cx = o.cx != null ? o.cx : o.x - 14, cy = o.cy != null ? o.cy : o.y - 16;
    return `<div class="dghost" style="left:${o.x}px;top:${o.y}px;${o.w ? `width:${o.w}px` : ''}">${stack}<div class="g1">${o.lead || (o.dev ? TL.dm(o.dev) : '')}<span class="nm">${o.name}</span>${o.chain ? TP.miniChain(o.chain) : ''}</div>${g2}</div>` + TL.cursor(cx, cy);
  };
  TL.pop = function (o) {
    return `<div class="pop" style="left:${o.x}px;top:${o.y}px;${o.w ? `width:${o.w}px` : ''}"><span class="pa" style="top:${o.arrow || 24}px"></span>
      <div class="ph"><div class="pt">${o.title}</div>${o.desc ? `<div class="pd">${o.desc}</div>` : ''}</div>
      ${o.rows ? `<div class="pb"><div class="outc">${o.rows.map(r => `<div class="orow ${r.z ? 'z' : ''}"><span class="ic" ${r.c ? `style="color:${r.c}"` : ''}>${I(r.i, 's14')}</span><span>${r.t}</span></div>`).join('')}</div></div>` : ''}
      <div class="pf"><span class="note">${o.note || ''}</span>${o.actions}</div></div>`;
  };

  /* ---------- the HX editor, folded under the pane ---------- */
  TL.hxCells = (amp) => [{ t: 'fsw', on: true, led: 'var(--c-amp)', label: 'On/Off' }, { t: 'div' }, ...amp];
  // Lay out the block's face in whatever height the pane leaves: balanced
  // rows of the largest knobs that fit, or one row of 44 pt knobs that
  // scrolls sideways when even that does not.
  TL.fitFace = function (el, cells, opts = {}) {
    const pane = el.closest('.pane');
    const head = pane.querySelector('.bhd');
    const room = pane.clientHeight - (head ? head.offsetHeight : 0) - (opts.pad || 10);
    const width = el.parentElement.clientWidth - 40 - (opts.minus || 0);
    const style = 'margin:0 20px;' + (opts.style || '');
    for (const k of opts.sizes || [68, 60, 52, 44]) {
      const html = TP.face(cells, width, { sizes: [k], style });
      const probe = document.createElement('div');
      probe.style.cssText = 'position:absolute;visibility:hidden;left:0;top:0;width:' + (width + 40) + 'px';
      probe.innerHTML = html;
      document.body.appendChild(probe);
      const fh = probe.firstElementChild.offsetHeight;
      probe.remove();
      if (fh <= room) { el.outerHTML = html; return k; }
    }
    // Less room than one row: the face folds away and the head stays, with
    // its name and its lens switch. Dragging the pane down brings it back.
    if (room < 96) { el.outerHTML = ''; return -1; }
    // One row, scrolled: the face keeps its knobs at 44 pt and fades at the right.
    const row = TP.face(cells, 1e5, { sizes: [opts.rowSize || 44] });
    el.outerHTML = `<div class="facerow">${row}<div class="fadeR">${I('chevron-right')}</div></div>`;
    return 0;
  };

  /* ---------- the deck, as built: discard sits before undo ---------- */
  TL.deck = function (o) {
    const s = TL.size();
    let h = `<header class="deck">`;
    if (o.sideToggle) h += `<span class="iconbtn" style="margin-left:-8px">${I('panel-left')}</span>`;
    h += `<span class="slotchip num">${o.slot}</span>`;
    h += `<div class="ptitlewrap"><div class="ptitle">${o.name}</div><div class="pmeta">${o.meta || ''}</div></div>`;
    h += `<div class="grow"></div>`;
    if (o.snaps) h += `<div class="seg snapseg">${o.snaps.map((sn, i) => `<span class="s ${sn.on ? 'on' : ''}"><span class="n num">${i + 1}</span>${s === 's' && !sn.on ? '' : sn.name}</span>`).join('')}</div>`;
    if (o.tempo) h += `<div class="vsep"></div><div class="tempo"><span class="v num">${o.tempo}</span><span class="u">BPM</span></div><span class="btn sm">Tap</span>`;
    h += `<div class="vsep"></div>`;
    h += `<span class="iconbtn ${o.dirty ? '' : 'dis'}" title="Discard changes">${I('rotate-ccw')}</span>`;
    h += `<span class="iconbtn ${o.undo ? '' : 'dis'}">${I('undo-2')}</span><span class="iconbtn ${o.redo ? '' : 'dis'}">${I('redo-2')}</span>`;
    h += o.save || (o.dirty ? `<span class="btn primary">Save${s === 's' ? '' : '<span class="kbd">Ctrl S</span>'}</span>` : `<span class="btn disabled">${I('check')}Saved</span>`);
    return h + `</header>`;
  };
  TL.hxDeck = (o = {}) => TL.deck(Object.assign({
    slot: '01B', name: 'Plexi Crunch',
    meta: `<span class="hot"><span class="dot"></span>3 changes not saved</span>${TL.size() === 's' ? '' : '<span class="sep">·</span><span>In your library as v2</span>'}`,
    snaps: [{ name: 'Verse' }, { name: 'Chorus', on: true }, { name: 'Solo' }], tempo: '104.0', undo: true, dirty: true,
  }, o));
  TL.hxBoardHead = (extra = '') => `<span><b>7</b> blocks</span><span>·</span><span>Path 1</span>${extra}<span class="grow"></span>${TP.legend()}`;
  TL.bhead = (o) => `<div class="bhd"><span class="bwell">${G(o.cat || 'amp')}</span>
    <div style="min-width:0"><div class="bname">${o.name}<span class="chev">${I('chevron-down', 's14')}</span></div>
    <div class="bmeta"><span class="c">${o.catName || 'Amp'}</span><span>·</span><span>${o.meta || 'Guitar'}</span><span>·</span><span>Mono</span></div></div>
    ${TL.size() === 's' ? `<span class="btn sm" style="margin-left:6px">${I('layout-grid')}</span>` : `<span class="btn sm" style="margin-left:8px">${I('layout-grid')}Change model</span>`}
    <span class="iconbtn">${I('copy')}</span><span class="iconbtn dis">${I('clipboard-paste')}</span><span class="iconbtn">${I('trash-2')}</span>
    <div class="seg lens"><span class="s on">Block</span><span class="s">${TL.size() === 's' ? 'Switches' : 'Footswitches'}</span><span class="s">Snapshots</span></div></div>`;

  /* ---------- menus: the redesign's idiom, with keys and reasons ---------- */
  TL.menuRows = (rows) => rows.map(r => {
    if (r === '-') return `<div class="msep"></div>`;
    if (r.head) return `<div class="mhead"><span class="ell">${r.head}</span><span class="h">${r.h || ''}</span></div>`;
    if (r.note) return `<div style="padding:4px 10px 6px 35px;font-size:11.5px;line-height:16px;color:var(--muted);white-space:normal">${r.note}</div>`;
    const keys = r.k ? `<span class="k">${r.k.map(k => `<span class="kbdk">${k}</span>`).join('')}</span>` : '';
    const why = r.why ? `<span class="why">${r.why}</span>` : '';
    const hint = r.h ? `<span class="h">${r.h}</span>` : '';
    const sub = r.sub ? `<span class="h">${I('chevron-right', 's14')}</span>` : '';
    return `<div class="mrow ${r.cls || ''} ${r.srv ? 'srv' : ''}">${r.icon ? I(r.icon) : '<span style="width:15px;flex:none"></span>'}<span>${r.t}</span>${hint}${why}${keys}${sub}</div>`;
  }).join('');
  TL.menu = (o) => `<div class="menu" ${o.id ? `id="${o.id}"` : ''} style="left:${o.x || 0}px;top:${o.y || 0}px;width:${o.w || 260}px;${o.style || ''}">${TL.menuRows(o.rows)}</div>`;

  TL.mount = function (html, after) {
    if (after) TP.later(after);
    TP.mount(html);
  };
  window.TL = TL;
})();
