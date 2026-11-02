// DEADAIR GUI Desktop Client // Emoji-Free Sovereign Edition
(function () {
  let state = {
    activeTab: 'main', // 'main' | 'direct' | 'group'
    replyingTo: null,
    attachedImage: null, // { dataUri, name }
    stationNotes: [],
    directNotes: [],
    groupNotes: [],
    relays: [],
    peers: [],
    bookmarks: [],
    identity: { name: 'anon', pubkey: '', npub: '' },
    shareAddress: 'ws://127.0.0.1:4869',
    safeTicket: '',
    detectedIp: '127.0.0.1',
    relayActive: true,
    outboxCount: 0,
    autoScroll: true,
  };

  const AVATAR_COLORS = ['#00e5ff', '#e040fb', '#ffea00', '#00ff66', '#4da6ff', '#ff3366'];

  // DOM Elements
  const notesContainer = document.getElementById('notes-container');
  const noteInput = document.getElementById('note-input');
  const sendBtn = document.getElementById('send-btn');
  const tabMainBtn = document.getElementById('tab-main-btn');
  const tabDirectBtn = document.getElementById('tab-direct-btn');
  const tabGroupsBtn = document.getElementById('tab-groups-btn');
  const mainBadgeCount = document.getElementById('main-badge-count');
  const directBadgeCount = document.getElementById('direct-badge-count');
  const groupsBadgeCount = document.getElementById('groups-badge-count');
  const directBar = document.getElementById('direct-bar');
  const directRecipientInput = document.getElementById('direct-recipient-input');
  const groupBar = document.getElementById('group-bar');
  const groupChannelInput = document.getElementById('group-channel-input');
  const attachImgBtn = document.getElementById('attach-img-btn');
  const imageFileInput = document.getElementById('image-file-input');
  const imagePreviewBanner = document.getElementById('image-preview-banner');
  const imagePreviewThumb = document.getElementById('image-preview-thumb');
  const imagePreviewName = document.getElementById('image-preview-name');
  const cancelImageBtn = document.getElementById('cancel-image-btn');
  const sidebarToggleBtn = document.getElementById('sidebar-toggle-btn');
  const sidebar = document.getElementById('station-sidebar');
  const handleBadge = document.getElementById('handle-badge');
  const handleText = document.getElementById('handle-text');
  const relayPill = document.getElementById('relay-pill');
  const relayText = document.getElementById('relay-text');
  const ticketPill = document.getElementById('ticket-pill');
  const replyBanner = document.getElementById('reply-banner');
  const replyTargetName = document.getElementById('reply-target-name');
  const cancelReplyBtn = document.getElementById('cancel-reply-btn');
  const clearFeedBtn = document.getElementById('clear-feed-btn');
  const resumeStreamBtn = document.getElementById('resume-stream-btn');
  const streamLabel = document.getElementById('stream-label');
  const toastEl = document.getElementById('toast-notification');
  const specIp = document.getElementById('spec-ip');
  const specNpub = document.getElementById('spec-npub');
  const specOutbox = document.getElementById('spec-outbox');
  const peersList = document.getElementById('peers-list');
  const relaysList = document.getElementById('relays-list');
  const bmUrlInput = document.getElementById('bm-url-input');
  const addBmBtn = document.getElementById('add-bm-btn');

  // Modal elements
  const ticketModal = document.getElementById('ticket-modal');
  const modalTicketText = document.getElementById('modal-ticket-text');
  const modalCopyTicketBtn = document.getElementById('modal-copy-ticket-btn');
  const modalCloseTicketBtn = document.getElementById('modal-close-ticket-btn');
  const modalImportTicketInput = document.getElementById('modal-import-ticket-input');
  const modalConnectTicketBtn = document.getElementById('modal-connect-ticket-btn');

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
    // Safe ticket links
    escaped = escaped.replace(/(deadair:\/\/[a-zA-Z0-9+/=]+)/g, '<span class="safe-ticket-link" style="color: #ffea00; font-weight: bold; cursor: pointer;">$1</span>');
    // URLs
    escaped = escaped.replace(/(https?:\/\/[^\s]+)/g, '<a href="$1" target="_blank" rel="noopener">$1</a>');
    return escaped;
  }

  function renderNoteCard(note) {
    const card = document.createElement('article');
    card.className = 'note-card deadair-post';
    card.dataset.id = note.id;

    const color = AVATAR_COLORS[note.color_idx % AVATAR_COLORS.length] || '#00e5ff';

    // Picture check
    let imageHtml = '';
    let textContent = note.content;

    if (note.is_image || note.content.includes('data:image/') || note.content.startsWith('![')) {
      const match = note.content.match(/!\[(.*?)\]\((.*?)\)/);
      if (match) {
        const caption = match[1] || 'attached image';
        const src = match[2];
        imageHtml = `
          <div style="margin: 8px 0; border: 1px solid #282e3f; border-radius: 4px; overflow: hidden; max-width: 420px; background: #000;">
            <img src="${src}" alt="${escapeHtml(caption)}" style="width: 100%; max-height: 280px; object-fit: contain; display: block; cursor: pointer;" onclick="window.open(this.src)">
            <div style="font-size: 0.75rem; color: #7f8a9f; padding: 4px 8px; background: #141720;">PICTURE: ${escapeHtml(caption)}</div>
          </div>
        `;
        textContent = ''; // Picture displayed visually
      }
    }

    card.innerHTML = `
      <div class="note-header">
        <span class="author-dot" style="background-color: ${color};"></span>
        <span class="author-name">${escapeHtml(note.author_name)}</span>
        <span class="author-handle">${escapeHtml(note.handle)}</span>
        <span class="bullet">|</span>
        <span class="note-time" data-ts="${note.created_at}">${note.relative_time}</span>
        <span class="station-badge">${note.tab ? note.tab.toUpperCase() : 'DEADAIR'}</span>
      </div>
      ${imageHtml}
      ${textContent ? `<div class="note-content">${formatContent(textContent)}</div>` : ''}
      <div class="note-footer">
        <button class="action-btn btn-reply" data-id="${note.id}" data-pubkey="${note.pubkey}" data-author="${escapeHtml(note.author_name)}">
          [RE: <span>${note.replies}</span>] Reply
        </button>
        <span class="action-btn">[RP: <span>${note.reposts}</span>]</span>
        <button class="action-btn btn-like ${note.is_liked_by_me ? 'liked' : ''}" data-id="${note.id}" data-pubkey="${note.pubkey}">
          [+1: <span>${note.likes}</span>] ${note.is_liked_by_me ? 'Liked' : 'Like'}
        </button>
      </div>
    `;

    return card;
  }

  function getActiveNotes() {
    if (state.activeTab === 'direct') {
      return state.directNotes;
    } else if (state.activeTab === 'group') {
      return state.groupNotes;
    }
    return state.stationNotes;
  }

  function updateTimeline() {
    notesContainer.innerHTML = '';
    const currentList = getActiveNotes();

    if (currentList.length === 0) {
      let emptyMsg = 'No transmissions on DEADAIR main mesh yet. Broadcast below.';
      if (state.activeTab === 'direct') {
        emptyMsg = 'No direct private messages yet. Type recipient above and transmit.';
      } else if (state.activeTab === 'group') {
        emptyMsg = `No notes in #${escapeHtml(groupChannelInput.value)} group channel yet.`;
      }

      notesContainer.innerHTML = `
        <div class="empty-state" style="margin-top: 40px; font-size: 0.9rem;">
          ${emptyMsg}
        </div>
      `;
      return;
    }

    const fragment = document.createDocumentFragment();
    currentList.forEach((note) => {
      fragment.appendChild(renderNoteCard(note));
    });
    notesContainer.appendChild(fragment);

    if (state.autoScroll) {
      notesContainer.scrollTop = notesContainer.scrollHeight;
    }
  }

  function switchTab(newTab) {
    state.activeTab = newTab;
    tabMainBtn.classList.toggle('active', newTab === 'main');
    tabDirectBtn.classList.toggle('active', newTab === 'direct');
    tabGroupsBtn.classList.toggle('active', newTab === 'group');

    directBar.style.display = (newTab === 'direct') ? 'flex' : 'none';
    groupBar.style.display = (newTab === 'group') ? 'flex' : 'none';

    updateTimeline();
  }

  function updateSidebar() {
    specIp.textContent = state.detectedIp;
    specNpub.textContent = state.identity.npub ? (state.identity.npub.slice(0, 10) + '...' + state.identity.npub.slice(-4)) : 'offline';
    specOutbox.textContent = `${state.outboxCount} pending`;

    mainBadgeCount.textContent = state.stationNotes.length;
    directBadgeCount.textContent = state.directNotes.length;
    groupsBadgeCount.textContent = state.groupNotes.length;

    // Discovered Peers
    const peerCountEl = document.getElementById('peer-count');
    peerCountEl.textContent = state.peers.length;

    if (state.peers.length === 0) {
      peersList.innerHTML = '<div class="empty-state">Searching local UDP &amp; mesh relays...</div>';
    } else {
      peersList.innerHTML = '';
      state.peers.forEach((peer) => {
        const item = document.createElement('div');
        item.className = 'peer-item';
        item.style.padding = '6px 8px';
        item.style.borderBottom = '1px solid #1a1e2a';
        item.style.display = 'flex';
        item.style.justifyContent = 'space-between';
        item.style.alignItems = 'center';
        item.style.fontSize = '0.8rem';
        item.innerHTML = `
          <div>
            <span style="color: var(--accent-cyan); font-weight: bold;">${escapeHtml(peer.name)}</span>
            <div style="font-size: 0.72rem; color: var(--text-dim);">${escapeHtml(peer.ws_url)}</div>
          </div>
          <button class="btn-micro select-peer-btn" data-url="${escapeHtml(peer.ws_url)}" data-name="${escapeHtml(peer.name)}" style="font-size: 0.7rem; padding: 2px 6px;">DM</button>
        `;
        peersList.appendChild(item);
      });
    }

    // Connected Relays & Bookmarks
    const relayCountEl = document.getElementById('relay-count');
    relayCountEl.textContent = state.relays.length;

    if (state.relays.length === 0) {
      relaysList.innerHTML = '<div class="empty-state">No relays configured</div>';
    } else {
      relaysList.innerHTML = '';
      state.relays.forEach((relay) => {
        const item = document.createElement('div');
        item.className = 'relay-item';
        item.style.padding = '6px 8px';
        item.style.borderBottom = '1px solid #1a1e2a';
        item.style.display = 'flex';
        item.style.justifyContent = 'space-between';
        item.style.alignItems = 'center';
        item.style.fontSize = '0.8rem';
        const statusColor = relay.connected ? 'var(--accent-green)' : 'var(--accent-red)';
        const latency = relay.latency_ms ? ` (${relay.latency_ms}ms)` : '';
        item.innerHTML = `
          <div style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 170px;">
            <span style="color: ${statusColor}; font-weight: bold;">${relay.connected ? '[+]' : '[-]'}</span>
            <span style="color: #fff;">${escapeHtml(relay.url)}</span>
            <span style="font-size: 0.7rem; color: var(--text-dim);">${latency}</span>
          </div>
          <button class="btn-micro remove-bm-btn" data-url="${escapeHtml(relay.url)}" style="color: var(--accent-red); font-size: 0.7rem;">Del</button>
        `;
        relaysList.appendChild(item);
      });
    }
  }

  async function loadInitialState() {
    try {
      const data = await invoke('get_state');
      if (data) {
        state.identity.name = data.name;
        state.identity.pubkey = data.pubkey;
        state.identity.npub = data.npub;
        state.shareAddress = data.share_address;
        state.safeTicket = data.safe_ticket;
        state.detectedIp = data.detected_ip;
        state.relayActive = data.relay_active;
        state.outboxCount = data.outbox_count;
        state.stationNotes = data.station_notes || [];
        state.directNotes = data.direct_notes || [];
        state.groupNotes = data.group_notes || [];
        state.bookmarks = data.bookmarks || [];
        state.relays = data.relays || [];
        state.peers = data.peers || [];

        handleText.textContent = state.identity.name;
        relayText.textContent = state.relayActive ? `ACTIVE :${data.relay_port}` : 'OFF';
        relayPill.classList.toggle('active', state.relayActive);

        updateTimeline();
        updateSidebar();
      }
    } catch (e) {
      console.error('Failed to load initial state:', e);
    }
  }

  // Handle Send Note / Picture
  async function handleSend() {
    const text = noteInput.value.trim();

    if (!text && !state.attachedImage) {
      return;
    }

    sendBtn.disabled = true;
    try {
      const targetTab = state.activeTab;
      const groupName = groupChannelInput.value.trim();
      const recipient = directRecipientInput.value.trim();

      if (state.attachedImage) {
        await invoke('send_image', {
          dataUri: state.attachedImage.dataUri,
          caption: text || state.attachedImage.name,
          tab: targetTab,
          group: (targetTab === 'group') ? groupName : null,
          recipient: (targetTab === 'direct') ? recipient : null,
        });
        state.attachedImage = null;
        imagePreviewBanner.style.display = 'none';
      } else {
        await invoke('send_note', {
          text,
          replyToId: state.replyingTo ? state.replyingTo.id : null,
          replyToPubkey: state.replyingTo ? state.replyingTo.pubkey : null,
          tab: targetTab,
          group: (targetTab === 'group') ? groupName : null,
          recipient: (targetTab === 'direct') ? recipient : null,
        });
      }

      noteInput.value = '';
      cancelReply();
      showToast('Transmission broadcasted to mesh');
      await loadInitialState();
    } catch (e) {
      showToast(`Transmit error: ${e}`);
    } finally {
      sendBtn.disabled = false;
    }
  }

  function cancelReply() {
    state.replyingTo = null;
    replyBanner.style.display = 'none';
  }

  // Setup Event Listeners
  function initListeners() {
    tabMainBtn.addEventListener('click', () => switchTab('main'));
    tabDirectBtn.addEventListener('click', () => switchTab('direct'));
    tabGroupsBtn.addEventListener('click', () => switchTab('group'));

    sendBtn.addEventListener('click', handleSend);
    noteInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    });

    // Picture attachment
    attachImgBtn.addEventListener('click', () => {
      imageFileInput.click();
    });

    imageFileInput.addEventListener('change', (e) => {
      const file = e.target.files[0];
      if (!file) return;

      if (file.size > 2 * 1024 * 1024) {
        showToast('Image too large (>2MB). Please pick a smaller image.');
        return;
      }

      const reader = new FileReader();
      reader.onload = (event) => {
        state.attachedImage = {
          dataUri: event.target.result,
          name: file.name,
        };
        imagePreviewThumb.src = event.target.result;
        imagePreviewName.textContent = `${file.name} (${Math.round(file.size / 1024)} KB)`;
        imagePreviewBanner.style.display = 'flex';
      };
      reader.readAsDataURL(file);
    });

    cancelImageBtn.addEventListener('click', () => {
      state.attachedImage = null;
      imagePreviewBanner.style.display = 'none';
      imageFileInput.value = '';
    });

    // Handle rename station
    handleBadge.addEventListener('click', async () => {
      const newName = prompt('Enter new station handle (no spaces):', state.identity.name);
      if (newName && newName.trim()) {
        try {
          const res = await invoke('set_station_name', { newName: newName.trim() });
          state.identity.name = res;
          handleText.textContent = res;
          showToast(`Station handle updated to ${res}`);
        } catch (e) {
          showToast(`Error: ${e}`);
        }
      }
    });

    // Toggle relay
    relayPill.addEventListener('click', async () => {
      try {
        const active = await invoke('toggle_embedded_relay');
        state.relayActive = active;
        relayPill.classList.toggle('active', active);
        relayText.textContent = active ? 'ACTIVE :4869' : 'OFF';
        showToast(active ? 'Relay started on :4869' : 'Relay stopped');
      } catch (e) {
        showToast(`Relay error: ${e}`);
      }
    });

    // Safe Ticket Modal
    ticketPill.addEventListener('click', async () => {
      try {
        const ticket = await invoke('get_safe_ticket');
        state.safeTicket = ticket || state.shareAddress;
        modalTicketText.value = state.safeTicket;
        ticketModal.style.display = 'flex';
      } catch (e) {
        showToast(`Ticket error: ${e}`);
      }
    });

    modalCloseTicketBtn.addEventListener('click', () => {
      ticketModal.style.display = 'none';
    });

    modalCopyTicketBtn.addEventListener('click', () => {
      navigator.clipboard.writeText(modalTicketText.value);
      showToast('Safe ticket copied to clipboard');
    });

    modalConnectTicketBtn.addEventListener('click', async () => {
      const code = modalImportTicketInput.value.trim();
      if (!code) return;
      try {
        const ticket = await invoke('connect_safe_ticket', { code });
        showToast(`Connected & bookmarked: ${ticket.name} [${ticket.ws}]`);
        modalImportTicketInput.value = '';
        ticketModal.style.display = 'none';
        await loadInitialState();
      } catch (e) {
        showToast(`Connection failed: ${e}`);
      }
    });

    // Add Bookmark
    addBmBtn.addEventListener('click', async () => {
      const val = bmUrlInput.value.trim();
      if (!val) return;
      try {
        await invoke('add_bookmark', { target: val, name: null });
        bmUrlInput.value = '';
        showToast('Node connected & added to bookmarks');
        await loadInitialState();
      } catch (e) {
        showToast(`Bookmark error: ${e}`);
      }
    });

    // Clear feed
    clearFeedBtn.addEventListener('click', async () => {
      await invoke('clear_timeline');
      state.stationNotes = [];
      state.directNotes = [];
      state.groupNotes = [];
      updateTimeline();
      showToast('Feed cleared');
    });

    cancelReplyBtn.addEventListener('click', cancelReply);

    // Sidebar Mobile toggle
    sidebarToggleBtn.addEventListener('click', () => {
      sidebar.classList.toggle('active');
    });

    // Delegated click for Likes and Replies
    notesContainer.addEventListener('click', async (e) => {
      const likeBtn = e.target.closest('.btn-like');
      if (likeBtn) {
        const id = likeBtn.dataset.id;
        const pubkey = likeBtn.dataset.pubkey;
        try {
          const liked = await invoke('like_note', { eventIdHex: id, authorPubkeyHex: pubkey });
          if (liked) {
            showToast('Liked [+1]');
            await loadInitialState();
          }
        } catch (err) {
          showToast(`Like error: ${err}`);
        }
        return;
      }

      const replyBtn = e.target.closest('.btn-reply');
      if (replyBtn) {
        state.replyingTo = {
          id: replyBtn.dataset.id,
          pubkey: replyBtn.dataset.pubkey,
          author: replyBtn.dataset.author,
        };
        replyTargetName.textContent = `@${state.replyingTo.author}`;
        replyBanner.style.display = 'flex';
        noteInput.focus();
        return;
      }
    });

    // Delegated click for Peer DM button
    peersList.addEventListener('click', (e) => {
      const dmBtn = e.target.closest('.select-peer-btn');
      if (dmBtn) {
        switchTab('direct');
        directRecipientInput.value = dmBtn.dataset.name || dmBtn.dataset.url;
        noteInput.focus();
      }
    });

    // Delegated click for Remove Bookmark
    relaysList.addEventListener('click', async (e) => {
      const removeBtn = e.target.closest('.remove-bm-btn');
      if (removeBtn) {
        const url = removeBtn.dataset.url;
        try {
          await invoke('remove_bookmark', { url });
          showToast(`Removed ${url}`);
          await loadInitialState();
        } catch (err) {
          showToast(`Error: ${err}`);
        }
      }
    });

    // Listen for Tauri backend broadcast events
    if (isTauri && window.__TAURI__.event) {
      window.__TAURI__.event.listen('station_note', (event) => {
        state.stationNotes.push(event.payload);
        if (state.activeTab === 'main') updateTimeline();
      });

      window.__TAURI__.event.listen('direct_note', (event) => {
        state.directNotes.push(event.payload);
        showToast(`New Direct Message from ${event.payload.author_name}`);
        if (state.activeTab === 'direct') updateTimeline();
        directBadgeCount.textContent = state.directNotes.length;
      });

      window.__TAURI__.event.listen('group_note', (event) => {
        state.groupNotes.push(event.payload);
        if (state.activeTab === 'group') updateTimeline();
        groupsBadgeCount.textContent = state.groupNotes.length;
      });

      window.__TAURI__.event.listen('peer_discovered', (event) => {
        const peer = event.payload;
        if (!state.peers.some((p) => p.ws_url === peer.ws_url)) {
          state.peers.push(peer);
          showToast(`Mesh peer discovered: ${peer.name}`);
          updateSidebar();
        }
      });

      window.__TAURI__.event.listen('ip_updated', (event) => {
        const [ip, share] = event.payload;
        state.detectedIp = ip;
        state.shareAddress = share;
        updateSidebar();
      });
    }
  }

  // Initialize
  document.addEventListener('DOMContentLoaded', () => {
    loadInitialState();
    initListeners();
  });
})();
