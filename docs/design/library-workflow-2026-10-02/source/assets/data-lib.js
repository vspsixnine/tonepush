/* Synthetic content for the library workflow mockups. Tone, setlist, song,
   artist and creator names are invented; HX model names are HX Edit's. Loaded
   after the redesign's data.js, whose presets and chains it reuses. */
(function () {
  const L = {};
  // Device markers: the family, then the model. An HX tone names the pedal it
  // was kept from; a PRO tone names the firmware its chain needs.
  L.DEV = {
    stomp: { f: 'HX', m: 'Stomp', name: 'HX Stomp' },
    xl: { f: 'HX', m: 'Stomp XL', name: 'HX Stomp XL' },
    fx: { f: 'HX', m: 'Effects', name: 'HX Effects' },
    helix: { f: 'HX', m: 'Helix LT', name: 'Helix LT' },
    pro2: { f: 'PRO', m: '2.x', name: 'StompStation PRO, firmware 2.x' },
    pro15: { f: 'PRO', m: '1.5', name: 'StompStation PRO, firmware 1.5' },
  };
  const d = L.DEV;

  /* The library, all pedals, by name. pedal: the slot holding it on the HX
     Stomp; sync: same or differs; cloud: published, changed (the library has a
     newer version) or null. */
  L.tones = [
    { name: 'Ambient Swell', dev: d.stomp, chain: ['vol', 'mod', 'delay', 'delay', 'reverb'], song: 'Original', char: 'Clean', rating: 3, added: '10 Sep', ver: 'v1', pedal: '02B', sync: 'same', cloud: null },
    { name: 'Big Room Lead', dev: d.xl, chain: ['dist', 'amp', 'cab', 'delay', 'reverb'], song: 'Neon Avenue', artist: 'June Arcade', char: 'Hi-gain', rating: 4, added: '22 Sep', ver: 'v2', pedal: null, cloud: 'published' },
    { name: 'Brown Lead', dev: d.stomp, chain: ['dist', 'amp', 'cab', 'delay', 'reverb'], song: 'Static Bloom', artist: 'June Arcade', char: 'Hi-gain', rating: 4, added: '11 Sep', ver: 'v3', pedal: '01C', sync: 'same', cloud: 'published' },
    { name: 'Doom Fuzz', dev: d.stomp, chain: ['dist', 'dist', 'amp', 'cab', 'reverb!'], song: 'Iron Valley', artist: 'Slow Comet', char: 'Fuzz', rating: 3, added: '15 Sep', ver: 'v1', pedal: '03A', sync: 'same', cloud: null },
    { name: 'Dream Pop', dev: d.stomp, chain: ['mod', 'amp', 'cab', 'delay', 'reverb'], song: 'Glasshouse', artist: 'Slow Comet', char: 'Clean', rating: 4, added: '23 Sep', ver: 'v1', pedal: '13A', sync: 'same', cloud: 'published' },
    { name: 'Edge of Breakup', dev: d.stomp, chain: ['dyn', 'dist', 'amp', 'cab', 'reverb'], song: 'Original', char: 'Drive', rating: 4, added: '14 Sep', ver: 'v1', pedal: '02A', sync: 'same', cloud: null },
    { name: 'Funk Rhythm', dev: d.stomp, chain: ['dyn', 'filter', 'amp', 'cab', 'reverb'], song: 'Original', char: 'Clean', rating: 3, added: '18 Sep', ver: 'v1', pedal: '03C', sync: 'same', cloud: null },
    { name: 'Garage Grit', dev: d.stomp, chain: ['dist', 'amp', 'cab'], song: 'Original', char: 'Drive', rating: 3, added: '24 Sep', ver: 'v1', pedal: null, cloud: null },
    { name: 'Glass Clean', dev: d.stomp, chain: ['dyn', 'amp', 'cab', 'mod', 'delay', 'reverb'], song: 'Harbour Lights', artist: 'The Night Signals', char: 'Clean', rating: 4, added: '10 Sep', ver: 'v1', pedal: '01A', sync: 'same', cloud: 'published' },
    { name: 'Glass Wall', dev: d.pro15, chain: ['dyn', 'amp', 'ir', 'eq', 'mod', 'delay', 'reverb'], song: 'Original', char: 'Clean', rating: 4, added: '29 Sep', ver: 'v1', pedal: null, cloud: null },
    { name: 'Octave Fuzz', dev: d.stomp, chain: ['pitch', 'dist', 'amp', 'cab'], song: 'Original', char: 'Fuzz', rating: 2, added: '21 Sep', ver: 'v1', pedal: '04C', sync: 'same', cloud: null },
    { name: 'Pedalboard Wash', dev: d.fx, chain: ['dyn', 'mod', 'delay', 'delay', 'reverb'], song: 'Original', char: 'Clean', rating: 3, added: '26 Sep', ver: 'v1', pedal: null, cloud: null },
    { name: 'Plexi Crunch', dev: d.stomp, chain: ['wah!', 'dist', 'amp', ['cab', 'cab'], 'delay!', 'reverb'], song: 'Original', char: 'Drive', rating: 4, added: '12 Sep', ver: 'v2', pedal: '01B', sync: 'same', cloud: 'published' },
    { name: 'Shimmer Lead', dev: d.pro2, chain: ['dyn', 'dist', 'amp', 'ir', 'delay', 'reverb'], song: 'Low Tide', artist: 'Marlow Kent', char: 'Hi-gain', rating: 4, added: '29 Sep', ver: 'v2', pedal: null, cloud: null },
    { name: 'Slapback Twang', dev: d.stomp, chain: ['dyn', 'amp', 'cab', 'delay', 'reverb'], song: 'Dust Road', artist: 'The Night Signals', char: 'Clean', rating: 0, added: '17 Sep', ver: 'v3', pedal: null, cloud: 'changed' },
    { name: 'Surf Spring', dev: d.stomp, chain: ['amp', 'cab', 'mod', 'reverb'], song: 'Original', char: 'Clean', rating: 3, added: '20 Sep', ver: 'v1', pedal: '06A', sync: 'same', cloud: null },
    { name: 'Tape Echo Clean', dev: d.stomp, chain: ['amp', 'cab', 'delay', 'reverb'], song: 'Paper Boats', artist: 'June Arcade', char: 'Clean', rating: 4, added: '19 Sep', ver: 'v2', pedal: '04B', sync: 'differs', cloud: null },
    { name: 'Velvet Drive', dev: d.pro2, chain: ['dyn', 'dyn', 'wah!', 'dist', 'amp', 'ir', 'eq', ['mod', 'mod!'], 'delay', 'reverb'], song: 'Low Tide', artist: 'Marlow Kent', char: 'Drive', rating: 5, added: '28 Sep', ver: 'v4', pedal: null, cloud: 'published' },
    { name: 'Worship Pad', dev: d.stomp, chain: ['vol', 'pitch', 'delay', 'reverb', 'reverb'], song: 'Original', char: 'Clean', rating: 5, added: '16 Sep', ver: 'v2', pedal: '03B', sync: 'differs', cloud: 'changed' },
  ];
  L.tone = (name) => L.tones.find(t => t.name === name);

  /* TonePush's public feed, for the HX Stomp. */
  L.cloud = [
    { name: 'Glass Cathedral', dev: d.stomp, chain: ['dyn', 'amp', 'cab', 'mod', 'delay', 'reverb'], song: 'Original', by: 'Mira Holt', char: 'Clean', downloads: '3,410', updated: '30 Sep', ver: 'v2 of 2', lib: false },
    { name: 'Midnight City', dev: d.stomp, chain: ['dyn', 'dist', 'amp', 'cab', 'delay', 'reverb'], song: 'Neon Rain', artist: 'Halcyon Drive', by: 'Tomas Reyes', char: 'Drive', downloads: '2,184', updated: '27 Sep', ver: 'v1', lib: false },
    { name: 'Stadium Rhythm', dev: d.stomp, chain: ['dyn', 'dist', 'amp', ['cab', 'cab'], 'reverb'], song: 'Gold Coast', artist: 'The Velvet Static', by: 'Ines Park', char: 'Hi-gain', downloads: '1,902', updated: '25 Sep', ver: 'v3 of 3', lib: true },
    { name: 'Velvet Fuzz', dev: d.stomp, chain: ['dist', 'amp', 'cab', 'mod', 'reverb'], song: 'Original', by: 'Dario Venn', char: 'Fuzz', downloads: '1,288', updated: '24 Sep', ver: 'v1', lib: false },
    { name: 'Warm Jazz Box', dev: d.stomp, chain: ['dyn', 'amp', 'cab', 'reverb'], song: 'Blue Hour', artist: 'Paper Moons', by: 'Ruth Okafor', char: 'Clean', downloads: '812', updated: '21 Sep', ver: 'v1', lib: false },
    { name: 'Chime Machine', dev: d.stomp, chain: ['dyn', 'amp', 'cab', 'delay', 'delay', 'reverb'], song: 'Original', by: 'Mira Holt', char: 'Clean', downloads: '655', updated: '19 Sep', ver: 'v2 of 2', lib: false },
    { name: 'Dust Devil', dev: d.stomp, chain: ['pitch', 'dist', 'amp', 'cab', 'delay'], song: 'Red Mesa', artist: 'Halcyon Drive', by: 'K. Albrecht', char: 'Fuzz', downloads: '590', updated: '18 Sep', ver: 'v1', lib: false },
    { name: 'Hollow Body Lead', dev: d.stomp, chain: ['dyn', 'dist', 'amp', 'cab', 'delay', 'reverb'], song: 'Original', by: 'Ana Salgado', char: 'Drive', downloads: '418', updated: '15 Sep', ver: 'v1', lib: false },
    { name: 'Tidal Swell', dev: d.stomp, chain: ['vol', 'mod', 'delay', 'reverb', 'reverb'], song: 'Original', by: 'Tomas Reyes', char: 'Clean', downloads: '377', updated: '12 Sep', ver: 'v1', lib: false },
  ];

  /* What this library published, for the Mine view. lib: the library's
     version; up: the version on TonePush. */
  L.mine = [
    { name: 'Glass Clean', dev: d.stomp, chain: ['dyn', 'amp', 'cab', 'mod', 'delay', 'reverb'], song: 'Harbour Lights', artist: 'The Night Signals', up: 'v1', lib: 'v1', downloads: '2,047', published: '10 Sep', state: 'same' },
    { name: 'Velvet Drive', dev: d.pro2, chain: ['dyn', 'dyn', 'wah!', 'dist', 'amp', 'ir', 'eq', ['mod', 'mod!'], 'delay', 'reverb'], song: 'Low Tide', artist: 'Marlow Kent', up: 'v4', lib: 'v4', downloads: '1,630', published: '28 Sep', state: 'same' },
    { name: 'Plexi Crunch', dev: d.stomp, chain: ['wah!', 'dist', 'amp', ['cab', 'cab'], 'delay!', 'reverb'], song: 'Original', up: 'v2', lib: 'v2', downloads: '1,284', published: '12 Sep', state: 'same' },
    { name: 'Brown Lead', dev: d.stomp, chain: ['dist', 'amp', 'cab', 'delay', 'reverb'], song: 'Static Bloom', artist: 'June Arcade', up: 'v3', lib: 'v3', downloads: '980', published: '11 Sep', state: 'same' },
    { name: 'Worship Pad', dev: d.stomp, chain: ['vol', 'pitch', 'delay', 'reverb', 'reverb'], song: 'Original', up: 'v1', lib: 'v2', downloads: '721', published: '16 Sep', state: 'newer' },
    { name: 'Dream Pop', dev: d.stomp, chain: ['mod', 'amp', 'cab', 'delay', 'reverb'], song: 'Glasshouse', artist: 'Slow Comet', up: 'v1', lib: 'v1', downloads: '455', published: '23 Sep', state: 'same' },
    { name: 'Big Room Lead', dev: d.xl, chain: ['dist', 'amp', 'cab', 'delay', 'reverb'], song: 'Neon Avenue', artist: 'June Arcade', up: 'v2', lib: 'v2', downloads: '388', published: '22 Sep', state: 'same' },
    { name: 'Slapback Twang', dev: d.stomp, chain: ['dyn', 'amp', 'cab', 'delay', 'reverb'], song: 'Dust Road', artist: 'The Night Signals', up: 'v2', lib: 'v3', downloads: '312', published: '17 Sep', state: 'newer' },
  ];

  L.setlists = [
    { name: 'Album release show', venue: 'Lido Rooftop, Berlin', date: '4 Oct 2026', dev: d.stomp, count: 42, ver: 'v2', state: '6 slots differ' },
    { name: 'Rehearsal', venue: 'Room 3', date: '28 Sep 2026', dev: d.stomp, count: 38, ver: 'v1', state: 'matches the pedal' },
    { name: 'Studio session', venue: 'Kranhaus Studio', date: '12 Sep 2026', dev: d.stomp, count: 24, ver: 'v1', state: '19 slots differ' },
    { name: 'Summer tour', venue: 'Various', date: '2 Aug 2026', dev: d.pro2, count: 21, ver: 'v3', state: 'for another pedal' },
    { name: 'Acoustic night', venue: 'Café Wendel', date: '18 Jul 2026', dev: d.stomp, count: 12, ver: 'v1', state: '30 slots differ' },
  ];

  /* Dream Pop as the HX Stomp plays it: the audition's chain. */
  L.dreamPopChain = () => ({
    kind: 'hx',
    input: { label: 'In', sub: 'Multi' }, output: { label: 'Out', sub: 'Multi' },
    items: [
      { t: 'block', b: { name: 'Optical Trem', cat: 'mod', tr: [{ fs: 'FS1', led: 'blue' }] } },
      { t: 'block', b: { name: 'US Deluxe Nrm', cat: 'amp', sel: true } },
      { t: 'block', b: { name: '1x12 US Deluxe', cat: 'cab' } },
      { t: 'block', b: { name: 'Adriatic Delay', cat: 'delay', stereo: true, tr: [{ fs: 'FS2', led: 'green' }] } },
      { t: 'block', b: { name: 'Ganymede', cat: 'reverb', stereo: true, tr: [{ fs: 'FS3', led: 'turquoise' }] } },
    ],
  });
  L.deluxe = [
    { label: 'Drive', val: '3.5', frac: 0.35, def: 0.35 },
    { label: 'Bass', val: '5.0', frac: 0.50, def: 0.44 },
    { label: 'Mid', val: '6.5', frac: 0.65, def: 0.52 },
    { label: 'Treble', val: '6.0', frac: 0.60, def: 0.57 },
    { label: 'Presence', val: '3.0', frac: 0.30, def: 0.10 },
    { label: 'Ch Vol', val: '7.0', frac: 0.70, def: 0.85 },
    { label: 'Master', val: '10.0', frac: 1.0, def: 1.0 },
    { label: 'Sag', val: '6.0', frac: 0.6, def: 0.5 },
    { label: 'Hum', val: '5.0', frac: 0.5, def: 0.5 },
    { label: 'Ripple', val: '5.0', frac: 0.5, def: 0.5 },
    { label: 'Bias', val: '6.0', frac: 0.6, def: 0.6 },
    { label: 'Bias X', val: '5.0', frac: 0.5, def: 0.5 },
  ];

  /* Glass Cathedral, a public tone by Mira Holt, as the HX Stomp plays it. */
  L.cathedralChain = () => ({
    kind: 'hx',
    input: { label: 'In', sub: 'Multi' }, output: { label: 'Out', sub: 'Multi' },
    items: [
      { t: 'block', b: { name: 'Deluxe Comp', cat: 'dyn' } },
      { t: 'block', b: { name: 'US Princess', cat: 'amp', sel: true } },
      { t: 'block', b: { name: '1x12 Blue Bell', cat: 'cab' } },
      { t: 'block', b: { name: 'PlastiChorus', cat: 'mod', stereo: true, tr: [{ fs: 'FS1', led: 'blue' }] } },
      { t: 'block', b: { name: 'Simple Delay', cat: 'delay', stereo: true, tr: [{ fs: 'FS2', led: 'green' }] } },
      { t: 'block', b: { name: 'Glitz', cat: 'reverb', stereo: true, tl: [{ icon: 'camera' }], tr: [{ fs: 'FS3', led: 'turquoise' }] } },
    ],
  });
  L.princess = [
    { label: 'Drive', val: '4.0', frac: 0.40, def: 0.35 },
    { label: 'Bass', val: '4.5', frac: 0.45, def: 0.44 },
    { label: 'Mid', val: '5.5', frac: 0.55, def: 0.52 },
    { label: 'Treble', val: '6.5', frac: 0.65, def: 0.57 },
    { label: 'Presence', val: '2.5', frac: 0.25, def: 0.10 },
    { label: 'Ch Vol', val: '6.5', frac: 0.65, def: 0.85 },
    { label: 'Master', val: '8.0', frac: 0.8, def: 1.0 },
    { label: 'Sag', val: '5.0', frac: 0.5, def: 0.5 },
    { label: 'Hum', val: '5.0', frac: 0.5, def: 0.5 },
    { label: 'Ripple', val: '5.0', frac: 0.5, def: 0.5 },
    { label: 'Bias', val: '5.5', frac: 0.55, def: 0.6 },
    { label: 'Bias X', val: '5.0', frac: 0.5, def: 0.5 },
  ];

  /* The StompStation PRO's 2.x chain as the editor draws it: fourteen
     positions, three fixed, the delay and the reverb in parallel. */
  L.proChain = (sel) => ({
    kind: 'pro',
    input: { label: 'In', sub: '' }, output: { label: 'Out', sub: '' },
    items: [
      { t: 'block', b: { name: 'Gate', sub: 'Noise gate', cat: 'dyn' } },
      { t: 'block', b: { name: 'Pitch', sub: 'Octave', cat: 'pitch', on: false } },
      { t: 'block', b: { name: 'Comp', sub: 'Studio', cat: 'dyn' } },
      { t: 'block', b: { name: 'Drive', sub: 'TS808 · Drive 9', cat: 'dist' } },
      { t: 'block', b: { name: 'Amp', sub: 'Recto Modern', cat: 'amp', lock: true, sel: sel === 'amp' } },
      { t: 'block', b: { name: 'IR', sub: 'V30 · SM57', cat: 'ir' } },
      { t: 'block', b: { name: 'EQ', sub: 'Parametric', cat: 'eq' } },
      { t: 'empty', slot: 9 },
      { t: 'block', b: { name: 'Mod', sub: 'Chorus', cat: 'mod', on: false } },
      { t: 'par', lanes: [[{ name: 'Delay', sub: 'Digital · 380 ms', cat: 'delay', lock: true }], [{ name: 'Reverb', sub: 'Shimmer', cat: 'reverb', lock: true }]] },
      { t: 'empty', slot: 14 },
    ],
  });
  window.L = L;
})();
