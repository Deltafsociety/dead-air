// DEADAIR GUI & Mobile Client
(function () {
  let state = {
    activeTab: 'station',
    replyingTo: null,
    stationNotes: [],
    publicNotes: [],
    relays: [],
    peers: [],
    bookmarks: [],
    identity: { name: 'anon', pubkey: '', npub: '' },
    shareAddress: 'ws://127.0.0.1:4869',
    detectedIp: '127.0.0.1',
    relayActive: true,
    publicActive: false,
    outboxCount: 0,
    autoScroll: true,
  };

  const AVATAR_COLORS = ['#00e5ff', '#e040fb', '#ffea00', '#00ff66', '#4da6ff', '#ff3366'];

  // DOM Elements
  const notesContainer = document.getElementById('notes-container');
  const noteInput = document.getElementById('note-input');
  const sendBtn = document.getElementById('send-btn');
  const tabStationBtn = document.getElementById('tab-station-btn');
  const tabPublicBtn = document.getElementById('tab-public-btn');
  const sidebarToggleBtn = document.getElementById('sidebar-toggle-btn');
  const sidebar = document.getElementById('station-sidebar');
  const handleBadge = document.getElementById('handle-badge');
  const handleText = document.getElementById('handle-text');
  const relayPill = document.getElementById('relay-pill');
  const relayText = document.getElementById('relay-text');
  const publicMeshPill = document.getElementById('public-mesh-pill');
  const publicMeshText = document.getElementById('public-mesh-text');
  const sharePill = document.getElementById('share-pill');
  const shareText = document.getElementById('share-text');
  const replyBanner = document.getElementById('reply-banner');
  const replyTargetName = document.getElementById('reply-target-name');
  const cancelReplyBtn = document.getElementById('cancel-reply-btn');
  const clearFeedBtn = document.getElementById('clear-feed-btn');
  const resumeStreamBtn = document.getElementById('resume-stream-btn');
  const streamLabel = document.getElementById('stream-label');
  const stationBadgeCount = document.getElementById('station-badge-count');
  const publicBadgeCount = document.getElementById('public-badge-count');
  const toastEl = document.getElementById('toast-notification');
  const specIp = document.getElementById('spec-ip');
  const specNpub = document.getElementById('spec-npub');
  const specOutbox = document.getElementById('spec-outbox');
  const peersList = document.getElementById('peers-list');
  const relaysList = document.getElementById('relays-list');
  const bmUrlInput = document.getElementById('bm-url-input');
  const addBmBtn = document.getElementById('add-bm-btn');

  const isTauri = typeof window !== 'undefined' && window.__TAURI__ && window.__TAURI__.core;

  async function invoke(cmd, args = {}) {
    if (isTauri) {
      return await window.__TAURI__.core.invoke(cmd, args);
    }
    console.warn(`[Mock IPC] ${cmd}:`, args);
    return null;
  }

  function showToast(msg, duration = 3000) {
    toastEl.textContent = msg;
    toastEl.style.display = 'block';
    clearTimeout(toastEl._timer);
    toastEl._timer = setTimeout(() => {
      toastEl.style.display = 'none';
    }, duration);
  }

  function formatRelativeTime(ts) {
    const now = Math.floor(Date.now() / 1000);
    if (ts > now) return 'just now';
    const diff = now - ts;
    if (diff < 10) return 'just now';
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  function formatContent(text) {
    let escaped = escapeHtml(text);
    // Hashtags
    escaped = escaped.replace(/(#[a-zA-Z0-9_]+)/g, '<span class="hashtag">$1</span>');
    // Mentions & npubs
    escaped = escaped.replace(/(@[a-zA-Z0-9_]+|nostr:npub1[a-z0-9]+|npub1[a-z0-9]+)/g, '<span class="mention">$1</span>');
    // URLs
    escaped = escaped.replace(/(https?:\/\/[^\s]+)/g, '<a href="$1" target="_blank" rel="noopener">$1</a>');
    return escaped;
  }

  function renderNoteCard(note) {
    const card = document.createElement('article');
    card.className = `note-card ${note.is_deadair ? 'deadair-post' : ''}`;
    card.dataset.id = note.id;

    const color = AVATAR_COLORS[note.color_idx % AVATAR_COLORS.length] || '#00e5ff';

    card.innerHTML = `
      <div class="note-header">
        <span class="author-dot" style="background-color: ${color};"></span>
        <span class="author-name">${escapeHtml(note.author_name)}</span>
        <span class="author-handle">${escapeHtml(note.handle)}</span>
        <span class="bullet">•</span>
        <span class="note-time" data-ts="${note.created_at}">${note.relative_time}</span>
        ${note.is_deadair ? '<span class="station-badge">DEADAIR</span>' : ''}
      </div>
      <div class="note-content">${formatContent(note.content)}</div>
      <div class="note-footer">
        <button class="action-btn btn-reply" data-id="${note.id}" data-pubkey="${note.pubkey}" data-author="${escapeHtml(note.author_name)}">
          💬 <span>${note.replies}</span> Reply
        </button>
        <span class="action-btn">🔁 <span>${note.reposts}</span></span>
        <button class="action-btn btn-like ${note.is_liked_by_me ? 'liked' : ''}" data-id="${note.id}" data-pubkey="${note.pubkey}">
          ❤️ <span>${note.likes}</span> ${note.is_liked_by_me ? 'Liked' : 'Like'}
        </button>
      </div>
    `;

    return card;
  }

  function updateTimeline() {
    const notes = state.activeTab === 'station' ? state.stationNotes : state.publicNotes;
    notesContainer.innerHTML = '';

    if (notes.length === 0) {
      notesContainer.innerHTML = `
        <div class="empty-state" style="margin-top: 40px; font-size: 0.9rem;">
          ${state.activeTab === 'station'
            ? 'No messages on domestic station yet. Type below to broadcast across your intranet.'
            : 'No public relay notes cached. Toggle [PUBLIC] to stream global notes.'}
        </div>
      `;
      return;
    }

    const fragment = document.createDocumentFragment();
    notes.forEach((note) => {
      fragment.appendChild(renderNoteCard(note));
    });
    notesContainer.appendChild(fragment);

    if (state.autoScroll) {
      notesContainer.scrollTop = notesContainer.scrollHeight;
    }
  }

  function updateSidebar() {
    specIp.textContent = state.detectedIp;
    specNpub.textContent = state.identity.npub || 'loading...';
    specOutbox.textContent = `${state.outboxCount} pending`;

    // Peers
    document.getElementById('peer-count').textContent = state.peers.length;
    if (state.peers.length === 0) {
      peersList.innerHTML = '<div class="empty-state">No UDP peers discovered yet</div>';
    } else {
      peersList.innerHTML = state.peers
        .map((p) => `<div class="peer-item"><span>● ${escapeHtml(p.name)}</span><span style="color:var(--text-dim);">${escapeHtml(p.ws_url)}</span></div>`)
        .join('');
    }

    // Relays
    document.getElementById('relay-count').textContent = state.relays.length;
    if (state.relays.length === 0) {
      relaysList.innerHTML = '<div class="empty-state">No relays connected</div>';
    } else {
      relaysList.innerHTML = state.relays
        .map((r) => `
          <div class="relay-item">
            <span style="color:${r.connected ? 'var(--accent-green)' : 'var(--text-dim)'};">
              ${r.connected ? '🟢' : '⚪'} ${escapeHtml(r.url)}
            </span>
            ${r.latency_ms != null ? `<span style="color:var(--text-dim);">${r.latency_ms}ms</span>` : ''}
          </div>
        `)
        .join('');
    }
  }

  function switchTab(tab) {
    state.activeTab = tab;
    tabStationBtn.classList.toggle('active', tab === 'station');
    tabPublicBtn.classList.toggle('active', tab === 'public');
    sidebar.classList.remove('open');
    updateTimeline();
  }

  function setReplyContext(id, pubkey, author) {
    state.replyingTo = { id, pubkey, author };
    replyTargetName.textContent = `@${author}`;
    replyBanner.style.display = 'flex';
    noteInput.focus();
  }

  function clearReplyContext() {
    state.replyingTo = null;
    replyBanner.style.display = 'none';
  }

  async function handleSend() {
    const text = noteInput.value.trim();
    if (!text) return;

    if (text.startsWith('/')) {
      handleCommand(text);
      noteInput.value = '';
      return;
    }

    try {
      const replyId = state.replyingTo ? state.replyingTo.id : null;
      const replyPk = state.replyingTo ? state.replyingTo.pubkey : null;

      const note = await invoke('send_note', {
        tab: state.activeTab,
        text,
        replyToId: replyId,
        replyToPubkey: replyPk,
      });

      noteInput.value = '';
      clearReplyContext();

      if (note) {
        if (state.activeTab === 'station') {
          state.stationNotes.push(note);
          stationBadgeCount.textContent = state.stationNotes.length;
        } else {
          state.publicNotes.push(note);
          publicBadgeCount.textContent = state.publicNotes.length;
        }
        updateTimeline();
        showToast('Transmitted note across network');
      }
    } catch (err) {
      showToast(`Error: ${err}`);
    }
  }

  async function handleCommand(cmd) {
    const parts = cmd.split(' ');
    if (parts[0] === '/name' && parts[1]) {
      const newName = parts.slice(1).join(' ');
      try {
        const updated = await invoke('set_station_name', { newName });
        state.identity.name = updated;
        handleText.textContent = updated;
        showToast(`Station handle updated to ${updated}`);
      } catch (err) {
        showToast(`Error: ${err}`);
      }
    } else if (parts[0] === '/clear') {
      await invoke('clear_timeline', { tab: state.activeTab });
      if (state.activeTab === 'station') state.stationNotes = [];
      else state.publicNotes = [];
      updateTimeline();
      showToast('Timeline cleared');
    } else if (parts[0] === '/help') {
      showToast('Commands: /name <handle> | /clear | /help');
    }
  }

  // Event Listeners
  sendBtn.addEventListener('click', handleSend);
  noteInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  });

  tabStationBtn.addEventListener('click', () => switchTab('station'));
  tabPublicBtn.addEventListener('click', () => switchTab('public'));
  sidebarToggleBtn.addEventListener('click', () => sidebar.classList.toggle('open'));

  cancelReplyBtn.addEventListener('click', clearReplyContext);

  clearFeedBtn.addEventListener('click', async () => {
    await invoke('clear_timeline', { tab: state.activeTab });
    if (state.activeTab === 'station') state.stationNotes = [];
    else state.publicNotes = [];
    updateTimeline();
  });

  resumeStreamBtn.addEventListener('click', () => {
    state.autoScroll = true;
    resumeStreamBtn.style.display = 'none';
    streamLabel.textContent = 'LIVE STREAMING';
    notesContainer.scrollTop = notesContainer.scrollHeight;
  });

  notesContainer.addEventListener('scroll', () => {
    const atBottom = notesContainer.scrollHeight - notesContainer.scrollTop <= notesContainer.clientHeight + 40;
    if (!atBottom && state.autoScroll) {
      state.autoScroll = false;
      resumeStreamBtn.style.display = 'inline-block';
      streamLabel.textContent = 'PAUSED';
    } else if (atBottom && !state.autoScroll) {
      state.autoScroll = true;
      resumeStreamBtn.style.display = 'none';
      streamLabel.textContent = 'LIVE STREAMING';
    }
  });

  notesContainer.addEventListener('click', async (e) => {
    const replyBtn = e.target.closest('.btn-reply');
    if (replyBtn) {
      const id = replyBtn.dataset.id;
      const pubkey = replyBtn.dataset.pubkey;
      const author = replyBtn.dataset.author;
      setReplyContext(id, pubkey, author);
      return;
    }

    const likeBtn = e.target.closest('.btn-like');
    if (likeBtn) {
      const id = likeBtn.dataset.id;
      const pubkey = likeBtn.dataset.pubkey;
      try {
        const liked = await invoke('like_note', { eventIdHex: id, authorPubkeyHex: pubkey });
        if (liked) {
          likeBtn.classList.add('liked');
          const countSpan = likeBtn.querySelector('span');
          if (countSpan) countSpan.textContent = parseInt(countSpan.textContent || '0', 10) + 1;
          showToast('Liked note ❤️');
        }
      } catch (err) {
        showToast(`Error: ${err}`);
      }
    }
  });

  handleBadge.addEventListener('click', async () => {
    const newName = prompt('Enter new station handle:', state.identity.name);
    if (newName && newName.trim()) {
      handleCommand(`/name ${newName.trim()}`);
    }
  });

  relayPill.addEventListener('click', async () => {
    try {
      const active = await invoke('toggle_embedded_relay');
      state.relayActive = active;
      relayPill.classList.toggle('active', active);
      relayText.textContent = active ? 'ACTIVE :4869' : 'OFF';
      showToast(active ? 'Relay started on :4869' : 'Relay stopped');
    } catch (err) {
      showToast(`Error: ${err}`);
    }
  });

  publicMeshPill.addEventListener('click', async () => {
    try {
      const active = await invoke('toggle_public_mesh');
      state.publicActive = active;
      publicMeshPill.classList.toggle('active', active);
      publicMeshText.textContent = active ? 'ON' : 'OFF';
      showToast(active ? 'Connected to global public relays' : 'Disconnected from public relays');
    } catch (err) {
      showToast(`Error: ${err}`);
    }
  });

  sharePill.addEventListener('click', () => {
    navigator.clipboard.writeText(state.shareAddress).then(() => {
      showToast(`Copied ${state.shareAddress} to clipboard`);
    }).catch(() => {
      showToast(state.shareAddress);
    });
  });

  addBmBtn.addEventListener('click', async () => {
    const url = bmUrlInput.value.trim();
    if (!url) return;
    try {
      const updated = await invoke('add_bookmark', { url, name: 'node' });
      state.bookmarks = updated;
      bmUrlInput.value = '';
      showToast(`Added bookmark ${url}`);
      loadState();
    } catch (err) {
      showToast(`Error: ${err}`);
    }
  });

  // Load initial state
  async function loadState() {
    try {
      const res = await invoke('get_state');
      if (!res) return;

      state.identity = { name: res.name, pubkey: res.pubkey, npub: res.npub };
      state.relayActive = res.relay_active;
      state.shareAddress = res.share_address;
      state.detectedIp = res.detected_ip;
      state.publicActive = res.public_relays_active;
      state.outboxCount = res.outbox_count;
      state.stationNotes = res.station_notes || [];
      state.publicNotes = res.public_notes || [];
      state.bookmarks = res.bookmarks || [];
      state.relays = res.relays || [];
      state.peers = res.peers || [];

      handleText.textContent = res.name;
      shareText.textContent = res.share_address;
      relayPill.classList.toggle('active', res.relay_active);
      relayText.textContent = res.relay_active ? `ACTIVE :${res.relay_port}` : 'OFF';
      publicMeshPill.classList.toggle('active', res.public_relays_active);
      publicMeshText.textContent = res.public_relays_active ? 'ON' : 'OFF';

      stationBadgeCount.textContent = state.stationNotes.length;
      publicBadgeCount.textContent = state.publicNotes.length;

      updateTimeline();
      updateSidebar();
    } catch (err) {
      console.error('Failed to load state:', err);
    }
  }

  // Setup Tauri Event Listeners
  if (isTauri && window.__TAURI__.event) {
    window.__TAURI__.event.listen('station_note', (event) => {
      state.stationNotes.push(event.payload);
      stationBadgeCount.textContent = state.stationNotes.length;
      if (state.activeTab === 'station') {
        notesContainer.appendChild(renderNoteCard(event.payload));
        if (state.autoScroll) notesContainer.scrollTop = notesContainer.scrollHeight;
      }
    });

    window.__TAURI__.event.listen('public_note', (event) => {
      state.publicNotes.push(event.payload);
      publicBadgeCount.textContent = state.publicNotes.length;
      if (state.activeTab === 'public') {
        notesContainer.appendChild(renderNoteCard(event.payload));
        if (state.autoScroll) notesContainer.scrollTop = notesContainer.scrollHeight;
      }
    });

    window.__TAURI__.event.listen('peer_discovered', (event) => {
      state.peers.push(event.payload);
      updateSidebar();
      showToast(`Discovered LAN peer ${event.payload.name}`);
    });

    window.__TAURI__.event.listen('ip_updated', (event) => {
      const [ip, share] = event.payload;
      state.detectedIp = ip;
      state.shareAddress = share;
      shareText.textContent = share;
      specIp.textContent = ip;
    });

    window.__TAURI__.event.listen('metrics_updated', () => {
      // Reload stats periodically or when metrics change
      loadState();
    });
  }

  // Periodic relative time update
  setInterval(() => {
    document.querySelectorAll('.note-time').forEach((el) => {
      const ts = parseInt(el.dataset.ts, 10);
      if (ts) el.textContent = formatRelativeTime(ts);
    });
  }, 10000);

  // Initialize
  loadState();
})();
