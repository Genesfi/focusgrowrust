// FocusGrow UI Logic, Notifications, Custom Local GIF Upload, Realtime Preview & Dynamic Scaling

document.addEventListener('DOMContentLoaded', () => {
    // UI Elements
    const setupView = document.getElementById('setup-view');
    const timerView = document.getElementById('timer-view');

    const pickerMinsInput = document.getElementById('picker-mins-input') || document.getElementById('picker-mins-display');
    const btnPickerUp = document.getElementById('btn-picker-up');
    const btnPickerDown = document.getElementById('btn-picker-down');

    const pickerBreakInput = document.getElementById('picker-break-input') || document.getElementById('picker-break-display');
    const btnBreakUp = document.getElementById('btn-break-up');
    const btnBreakDown = document.getElementById('btn-break-down');

    const pickerPeriodInput = document.getElementById('picker-period-input') || document.getElementById('picker-period-display');
    const btnPeriodUp = document.getElementById('btn-period-up');
    const btnPeriodDown = document.getElementById('btn-period-down');

    const breaksCountText = document.getElementById('breaks-count-text');
    const sessionPredictionBadge = document.getElementById('session-prediction-badge');
    const sessionPredictionText = document.getElementById('session-prediction-text');
    const predictionIconClock = document.getElementById('prediction-icon-clock');
    const predictionIconMoon = document.getElementById('prediction-icon-moon');
    const chkSkipBreaks = document.getElementById('chk-skip-breaks');
    const btnStartFocus = document.getElementById('btn-start-focus');

    const focusPeriodTitle = document.getElementById('focus-period-title');
    const activeTimerDisplay = document.getElementById('active-timer-display');
    const activeStatusLabel = document.getElementById('active-status-label');
    const btnPause = document.getElementById('btn-pause');
    const btnStop = document.getElementById('btn-stop');
    const pauseIcon = document.getElementById('pause-icon');
    const playIcon = document.getElementById('play-icon');
    const upNextText = document.getElementById('up-next-text');
    const upNextContainer = document.getElementById('up-next-container');
    const selectTimerSubtitleMode = document.getElementById('select-timer-subtitle-mode');
    let latestSessionData = null;
    const gaugeProgressBar = document.getElementById('gauge-progress-bar');
    const gaugeTicks = document.getElementById('gauge-ticks');

    const gaugeGifContainer = document.getElementById('gauge-gif-container');
    const gaugeGifImg = document.getElementById('gauge-gif-img');
    const cardGifContainer = document.getElementById('card-gif-container');
    const cardGifImg = document.getElementById('card-gif-img');

    const statYesterday = document.getElementById('stat-yesterday');
    const statGoalHours = document.getElementById('stat-goal-hours');
    const statStreak = document.getElementById('stat-streak');
    const statCompletedMins = document.getElementById('stat-completed-mins');
    const goalDonutFill = document.getElementById('goal-donut-fill');
    const goalDonutBg = document.getElementById('goal-donut-bg');
    const goalDonutContainer = document.getElementById('goal-donut-container');
    const donutTierBadge = document.getElementById('donut-tier-badge');

    const appListContainer = document.getElementById('app-list-container');
    const appSearchInput = document.getElementById('app-search-input');
    const btnRefreshApps = document.getElementById('btn-refresh-apps');

    // Modals & Settings
    const btnEditGoal = document.getElementById('btn-edit-goal');
    const goalModal = document.getElementById('goal-modal');
    const btnCloseGoalModal = document.getElementById('btn-close-goal-modal');

    const btnOptions = document.getElementById('btn-options');
    const optionsModal = document.getElementById('options-modal');
    const btnCloseOptionsModal = document.getElementById('btn-close-options-modal');
    const btnResetProgress = document.getElementById('btn-reset-progress');
    const btnClearWhitelist = document.getElementById('btn-clear-whitelist');
    const chkNotificationsToggle = document.getElementById('chk-notifications-toggle');

    const btnSelectCustomGif = document.getElementById('btn-select-custom-gif');
    const gifFileInput = document.getElementById('gif-file-input');
    const customGifNameDisplay = document.getElementById('custom-gif-name');
    const gifPreviewWrapper = document.getElementById('gif-preview-wrapper');
    const gifPreviewContainer = document.getElementById('gif-preview-container');
    const gifPreviewImg = document.getElementById('gif-preview-img');
    const gifStyleSection = document.getElementById('gif-style-section');
    const gifOpacityRow = document.getElementById('gif-opacity-row');
    const gifOpacitySlider = document.getElementById('gif-opacity-slider');
    const opacityLabel = document.getElementById('opacity-label');

    // Titlebar Controls
    const handleCloseClick = (e) => {
        const forceExit = !!(e.ctrlKey || e.metaKey);
        sendToCpp({ action: 'close', forceExit: forceExit });
    };

    let isExplicitlyMinimizing = false;
    document.getElementById('btn-min')?.addEventListener('click', () => {
        isExplicitlyMinimizing = true;
        setTimeout(() => { isExplicitlyMinimizing = false; }, 1200);
        sendToCpp({ action: 'minimize' });
    });
    document.getElementById('btn-max')?.addEventListener('click', () => sendToCpp({ action: 'maximize' }));
    document.getElementById('btn-close')?.addEventListener('click', handleCloseClick);
    document.getElementById('btn-pip-close')?.addEventListener('click', handleCloseClick);

    // Native Window Drag Handler
    const setupDragArea = (element) => {
        if (!element) return;
        element.addEventListener('mousedown', (e) => {
            if (e.button === 0 && !e.target.closest('button, input, label, a, .switch, .window-controls, #gauge-clickable-area, .gauge-center, .gauge-container, .timer-time, .up-next-info, .circle-btn, .icon-btn, .timer-action-bar')) {
                sendToCpp({ action: 'startDrag' });
            }
        });
    };

    document.querySelectorAll('.resize-handle').forEach(handle => {
        handle.addEventListener('mousedown', (e) => {
            if (e.button === 0) {
                e.preventDefault();
                e.stopPropagation();
                sendToCpp({ action: 'startResize', edge: handle.dataset.edge });
            }
        });
    });

    setupDragArea(document.getElementById('app-titlebar'));
    setupDragArea(document.querySelector('.titlebar'));
    setupDragArea(document.getElementById('focus-period-title'));
    setupDragArea(document.querySelector('#timer-view .card-header'));

    let lastPipToggleTime = 0;
    let pendingBlurAutoPipTimer = null;
    let isWindowCurrentlyFocused = true;
    let isFileInputActive = false;

    function handleWindowFocus() {
        isWindowCurrentlyFocused = true;
        isExplicitlyMinimizing = false;
        if (typeof checkAndHandleMidnightRollover === 'function') {
            checkAndHandleMidnightRollover();
        }
        if (pendingBlurAutoPipTimer) {
            clearTimeout(pendingBlurAutoPipTimer);
            pendingBlurAutoPipTimer = null;
        }
        setTimeout(() => { isFileInputActive = false; }, 400);
    }

    let isPipMorphing = false;

    function startPipTransition(isEnteringPip) {
        if (isPipMorphing) return;
        isPipMorphing = true;
        lastPipToggleTime = Date.now();

        if (pendingBlurAutoPipTimer) {
            clearTimeout(pendingBlurAutoPipTimer);
            pendingBlurAutoPipTimer = null;
        }

        // Auto-dismiss any active modals/settings immediately
        document.querySelectorAll('.modal-overlay.active, .modal.active').forEach(m => m.classList.remove('active'));

        // Step 1: Instantly tuck in / vanish vinyl overlay before any geometry or layout changes occur
        sendToCpp({
            action: 'updatePipVinylOverlay',
            visible: false
        });

        let targetW = isEnteringPip ? (userData.pipWidth || 195) : undefined;
        let targetH = isEnteringPip ? (userData.pipHeight || 280) : undefined;
        if (targetW && (targetW > 440 || targetW < 150)) targetW = 195;
        if (targetH && (targetH > 550 || targetH < 210)) targetH = 280;

        // Step 2: Smooth fade out current view (80ms)
        document.body.classList.remove('pip-morphing-in');
        document.body.classList.add('pip-morphing-out');

        setTimeout(() => {
            // Step 3: While view is completely hidden (opacity: 0), toggle pip-mode & request window resize
            if (isEnteringPip) {
                document.body.classList.add('pip-mode');
            } else {
                document.body.classList.remove('pip-mode');
            }

            sendToCpp({
                action: 'togglePip',
                enabled: isEnteringPip,
                width: targetW,
                height: targetH,
                hideTaskbar: !!userData.hideTaskbarInPip
            });

            // Step 4: Fade in the pristine new layout (120ms)
            requestAnimationFrame(() => {
                document.body.classList.remove('pip-morphing-out');
                document.body.classList.add('pip-morphing-in');

                setTimeout(() => {
                    document.body.classList.remove('pip-morphing-in');
                    isPipMorphing = false;

                    // Step 5: Now that window has arrived and settled, reveal vinyl overlay if music is playing
                    if (typeof updatePipPeekingVinyl === 'function') {
                        updatePipPeekingVinyl();
                    }
                    if (typeof syncVinylCenterPosition === 'function') {
                        syncVinylCenterPosition();
                    }
                }, 120);
            });
        }, 80);
    }

    function handlePipTransitionStart(isEntering, duration = 200) {
        startPipTransition(isEntering);
    }

    function handlePipTransitionEnd(isPip) {
        if (!isPipMorphing) {
            if (isPip) {
                document.body.classList.add('pip-mode');
            } else {
                document.body.classList.remove('pip-mode');
            }
            if (typeof updatePipPeekingVinyl === 'function') updatePipPeekingVinyl();
            if (typeof syncVinylCenterPosition === 'function') syncVinylCenterPosition();
        }
    }

    function handleWindowBlur() {
        isWindowCurrentlyFocused = false;
        if (isExplicitlyMinimizing) return;
        if (!userData.autoPipOnBlur) return;
        if (document.body.classList.contains('pip-mode')) return;
        if (isFileInputActive) return;
        if (isPipMorphing) return;

        // Only auto-pip if session is active or on timer view
        const isSessionActive = timerView?.classList.contains('active') || (activeState === 'focusing' || activeState === 'resting');
        if (!isSessionActive) return;

        // Auto-dismiss any active modals/settings so they don't get stuck in PiP
        document.querySelectorAll('.modal-overlay.active, .modal.active').forEach(m => m.classList.remove('active'));

        const elapsedSinceToggle = Date.now() - lastPipToggleTime;
        const debounceThreshold = 350;
        if (elapsedSinceToggle < debounceThreshold) {
            if (pendingBlurAutoPipTimer) clearTimeout(pendingBlurAutoPipTimer);
            const remainingWait = debounceThreshold - elapsedSinceToggle + 50;
            pendingBlurAutoPipTimer = setTimeout(() => {
                pendingBlurAutoPipTimer = null;
                if (!isWindowCurrentlyFocused && !document.body.classList.contains('pip-mode') && !isPipMorphing) {
                    handleWindowBlur();
                }
            }, remainingWait);
            return;
        }

        console.log(`[PIP] Auto switching to PIP on outside click/blur`);
        startPipTransition(true);
    }

    document.querySelectorAll('.btn-pip-toggle, #btn-pip-restore').forEach(btn => {
        btn.addEventListener('click', () => {
            const isCurrentlyPip = document.body.classList.contains('pip-mode');
            startPipTransition(!isCurrentlyPip);
        });
    });

    // Auto PiP on Window Blur (Click Outside Dashboard)
    document.addEventListener('click', (e) => {
        if (e.target && (e.target.tagName === 'INPUT' && e.target.type === 'file')) {
            isFileInputActive = true;
        }
    }, true);
    document.querySelectorAll('input[type="file"]').forEach(input => {
        input.addEventListener('click', () => { isFileInputActive = true; });
    });

    window.addEventListener('focus', handleWindowFocus);
    window.addEventListener('blur', handleWindowBlur);

    // Request Notification Permissions
    if ('Notification' in window && Notification.permission !== 'granted' && Notification.permission !== 'denied') {
        Notification.requestPermission();
    }

    // --- IndexedDB for Large GIF Storage ---
    const DB_NAME = 'FocusGrowDB';
    const DB_VERSION = 1;
    const STORE_NAME = 'gifs';

    const gifDb = {
        _db: null,
        init() {
            return new Promise((resolve, reject) => {
                const request = indexedDB.open(DB_NAME, DB_VERSION);
                request.onupgradeneeded = (e) => {
                    const db = e.target.result;
                    if (!db.objectStoreNames.contains(STORE_NAME)) {
                        db.createObjectStore(STORE_NAME, { keyPath: 'id' });
                    }
                };
                request.onsuccess = (e) => {
                    this._db = e.target.result;
                    resolve(this._db);
                };
                request.onerror = (e) => reject(e.target.error);
            });
        },
        async save(id, data) {
            if (!this._db) await this.init();
            return new Promise((resolve, reject) => {
                const tx = this._db.transaction(STORE_NAME, 'readwrite');
                const store = tx.objectStore(STORE_NAME);
                store.put({ id, data });
                tx.oncomplete = () => resolve();
                tx.onerror = (e) => reject(e.target.error);
            });
        },
        async get(id) {
            if (!id) return null;
            if (!this._db) await this.init();
            return new Promise((resolve, reject) => {
                const tx = this._db.transaction(STORE_NAME, 'readonly');
                const store = tx.objectStore(STORE_NAME);
                const request = store.get(id);
                request.onsuccess = () => resolve(request.result ? request.result.data : null);
                request.onerror = (e) => reject(e.target.error);
            });
        },
        async delete(id) {
            if (!this._db) await this.init();
            return new Promise((resolve, reject) => {
                const tx = this._db.transaction(STORE_NAME, 'readwrite');
                const store = tx.objectStore(STORE_NAME);
                store.delete(id);
                tx.oncomplete = () => resolve();
                tx.onerror = (e) => reject(e.target.error);
            });
        },
        async clearAll() {
            if (!this._db) await this.init();
            return new Promise((resolve, reject) => {
                const tx = this._db.transaction(STORE_NAME, 'readwrite');
                const store = tx.objectStore(STORE_NAME);
                store.clear();
                tx.oncomplete = () => resolve();
                tx.onerror = (e) => reject(e.target.error);
            });
        }
    };
    window.gifDb = gifDb;

    // Persistent State Management
    const STORAGE_KEY = 'focusgrow_user_data_v1';

    // Helper: Formats Date into local 'YYYY-MM-DD' (based on user's PC local clock, not UTC)
    function getLocalDateStr(d = new Date()) {
        const year = d.getFullYear();
        const month = String(d.getMonth() + 1).padStart(2, '0');
        const day = String(d.getDate()).padStart(2, '0');
        return `${year}-${month}-${day}`;
    }

    let todayDateStr = getLocalDateStr();
    const defaultIconSvg = `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="%2360cdff" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="4"/><line x1="9" y1="9" x2="15" y2="9"/><line x1="9" y1="13" x2="15" y2="13"/><line x1="9" y1="17" x2="12" y2="17"/></svg>`;
    const defaultVinylSvg = `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 500"><defs><radialGradient id="grooveGrad" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="%23111115"/><stop offset="25%" stop-color="%231a1a22"/><stop offset="35%" stop-color="%230d0d12"/><stop offset="50%" stop-color="%2322222a"/><stop offset="65%" stop-color="%230f0f14"/><stop offset="80%" stop-color="%231c1c24"/><stop offset="95%" stop-color="%230a0a0d"/><stop offset="100%" stop-color="%23050507"/></radialGradient><linearGradient id="sheen" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stop-color="rgba(255,255,255,0.18)"/><stop offset="45%" stop-color="rgba(255,255,255,0.02)"/><stop offset="50%" stop-color="rgba(255,255,255,0.22)"/><stop offset="55%" stop-color="rgba(255,255,255,0.02)"/><stop offset="100%" stop-color="rgba(255,255,255,0.12)"/></linearGradient><radialGradient id="centerLabel" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="%231e293b"/><stop offset="70%" stop-color="%230f172a"/><stop offset="100%" stop-color="%23020617"/></radialGradient><linearGradient id="goldAccent" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stop-color="%2360cdff"/><stop offset="100%" stop-color="%233b82f6"/></linearGradient></defs><circle cx="250" cy="250" r="248" fill="url(%23grooveGrad)" stroke="%232d3748" stroke-width="2"/><circle cx="250" cy="250" r="230" fill="none" stroke="rgba(255,255,255,0.04)" stroke-width="1.5"/><circle cx="250" cy="250" r="210" fill="none" stroke="rgba(0,0,0,0.4)" stroke-width="2"/><circle cx="250" cy="250" r="190" fill="none" stroke="rgba(255,255,255,0.05)" stroke-width="1.5"/><circle cx="250" cy="250" r="170" fill="none" stroke="rgba(0,0,0,0.5)" stroke-width="2"/><circle cx="250" cy="250" r="150" fill="none" stroke="rgba(255,255,255,0.04)" stroke-width="1.5"/><circle cx="250" cy="250" r="130" fill="none" stroke="rgba(0,0,0,0.5)" stroke-width="2"/><circle cx="250" cy="250" r="248" fill="url(%23sheen)"/><circle cx="250" cy="250" r="95" fill="url(%23centerLabel)" stroke="url(%23goldAccent)" stroke-width="3"/><circle cx="250" cy="250" r="88" fill="none" stroke="rgba(255,255,255,0.15)" stroke-dasharray="4 4" stroke-width="1"/><g transform="translate(250, 215) scale(1.3)" opacity="0.9"><path d="M-6 12 A6 6 0 1 1 -18 12 A6 6 0 1 1 -6 12 M-6 12 L-6 -10 L10 -15 L10 5 A6 6 0 1 1 -2 5 A6 6 0 1 1 10 5 L10 -15 L-6 -10" fill="none" stroke="%2360cdff" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></g><text x="250" y="272" font-family="'Segoe UI', sans-serif" font-size="14" font-weight="700" fill="%23ffffff" text-anchor="middle" letter-spacing="2">FOCUS %26 FLOW</text><text x="250" y="290" font-family="'Segoe UI', sans-serif" font-size="10" font-weight="600" fill="%2360cdff" text-anchor="middle" letter-spacing="1.5">HI-FI VINYL EDITION</text><text x="250" y="306" font-family="'Segoe UI', sans-serif" font-size="8" fill="%2394a3b8" text-anchor="middle" letter-spacing="1">PLAY MUSIC TO SYNC ALBUM ART</text><circle cx="250" cy="250" r="14" fill="%23020617" stroke="rgba(255,255,255,0.3)" stroke-width="1.5"/></svg>`;

    let userData = {
        selectedMins: 30,
        selectedPeriodMins: 25,
        selectedBreakMins: 5,
        dailyGoalHours: 1,
        completedMinutesToday: 0,
        yesterdayHours: 0.0,
        streakDays: 0,
        lastDateStr: todayDateStr,
        notificationsEnabled: true,
        gifOpacity: 78,
        gifDisplayMode: 'circle', // 'circle' or 'full'
        ambientMode: 'plant', // 'plant', 'custom', or 'ytmusic'
        accentMode: 'preset', // 'preset', 'custom', or 'ytmusic_dynamic'
        notificationSound: 'default', // 'default', 'reminder', 'alarm', 'chime', 'none'
        focusNotificationSound: 'default', // 'default', 'reminder', 'alarm', 'chime', 'custom', 'none'
        breakNotificationSound: 'chime',   // 'default', 'reminder', 'alarm', 'chime', 'custom', 'none'
        warningNotificationSound: 'reminder', // 'default', 'reminder', 'alarm', 'chime', 'custom', 'none'
        countdownSoundEnabled: true,
        countdownSoundStyle: 'zen', // 'zen' or 'digital'
        countdownDurationSec: 3, // 3, 5, or 10 seconds
        focusSoundFileName: '',
        breakSoundFileName: '',
        warningSoundFileName: '',
        customGifName: '',
        recentGifs: [], // Stores up to 5 recently used custom GIFs: { id, name, data }
        blockedApps: ['facebook.exe', 'tiktok.exe', 'instagram.exe'],
        restrictedSites: ['facebook.com', 'youtube.com', 'instagram.com', 'tiktok.com', 'twitter.com', 'x.com', 'reddit.com'],
        pipWidth: 195,
        pipHeight: 280,
        pipVinylWidth: 440,
        pipVinylHeight: 400,
        pipPeekingVinylEnabled: false,
        pipPeekingVinylSide: 'left',
        timerTheme: 'classic', // 'classic', 'hourglass', 'wave', 'blocks', 'dots', 'orbit'
        isStealthMode: false,
        stealthHoverPeek: true,
        showVinylSpindle: true,
        autoPipOnStart: false,
        autoPipOnBlur: false,
        hideTaskbarInPip: false,
        autoPauseEnabled: true,
        autoPauseSec: 15,
        useComplementaryColor: true,
        hourglassAutoRotate: false,
        completedMinutesByDate: {}, // { 'YYYY-MM-DD': minutes }
        prayerEnabled: true,
        prayerBreakEnabled: true,
        prayerAdvance: 5,
        prayerLat: -2.8554,
        prayerLng: 115.3283,
        prayerTz: 8,
        neutralOverstayEnabled: true,
        neutralOverstayIntervalMins: 30,
        neutralSoftBlockEnabled: true,
        timerSubtitleMode: 'countdown',
        ignoredNudgeStatsByDate: {}, // { 'YYYY-MM-DD': count }
        sessionMode: 'productive', // 'productive' or 'health'
        healthDailyGoalBreaks: 4,
        healthCompletedBreaksToday: 0,
        healthStreakDays: 0,
        healthStatsByDate: {}, // { 'YYYY-MM-DD': { breaks: 0 } }
        completedTimeFormat: 'smart', // 'smart' (e.g. 7h 30m), 'decimal' (7.5h), or 'minutes' (450m)
        autoCloseBlockedApps: false,
        blockAllMonitors: false // false: Active Monitor Only (default), true: All Monitors
    };

    let continuousNeutralSec = 0;
    let continuousProductiveSec = 0;
    let lastContinuousExe = '';
    let lastProductiveExe = '';
    let isNeutralSoftBlockActive = false;

    function isGoalMetWithTolerance(mins, targetMins, targetHours) {
        if (!mins || mins <= 0) return false;
        if (mins >= targetMins) return true;
        // Rounding tolerance: e.g. 238 mins -> (238/60).toFixed(1) = "4.0" which matches "4.0" target hours!
        const minsHoursFormatted = parseFloat((mins / 60).toFixed(1));
        const targetHoursFormatted = parseFloat(parseFloat(targetHours).toFixed(1));
        return minsHoursFormatted >= targetHoursFormatted;
    }

    function calculateStreakDays() {
        const dailyGoalHours = parseFloat(userData.dailyGoalHours) || 1;
        const dailyGoalMins = dailyGoalHours * 60;
        const healthGoalBreaks = userData.healthDailyGoalBreaks || 4;

        let streak = 0;

        // Check Today: ONLY increment if today's productive work goal is genuinely met!
        const todayMins = userData.completedMinutesToday || 0;
        const todayGoalMet = isGoalMetWithTolerance(todayMins, dailyGoalMins, dailyGoalHours);

        if (todayGoalMet) {
            streak += 1;
        }

        // Walk backwards starting from Yesterday (offset 1 day)
        const todayObj = new Date();
        for (let i = 1; i <= 365; i++) {
            const d = new Date();
            d.setDate(todayObj.getDate() - i);
            const dateStr = getLocalDateStr(d);

            let mins = (userData.completedMinutesByDate && userData.completedMinutesByDate[dateStr])
                ? userData.completedMinutesByDate[dateStr]
                : 0;

            if (mins === 0 && userData.appStatsByDate && userData.appStatsByDate[dateStr]) {
                const dayApps = userData.appStatsByDate[dateStr];
                let totalSec = 0;
                for (const exe in dayApps) {
                    totalSec += dayApps[exe];
                }
                mins = Math.floor(totalSec / 60);
            }

            // Health stand shield check for past day
            const dayHealthStats = (userData.healthStatsByDate && userData.healthStatsByDate[dateStr])
                ? userData.healthStatsByDate[dateStr]
                : null;
            const pastHealthBreaks = (dayHealthStats && typeof dayHealthStats.breaks === 'number') ? dayHealthStats.breaks : 0;
            const pastHealthShield = (pastHealthBreaks >= healthGoalBreaks);
            const pastWorkMet = isGoalMetWithTolerance(mins, dailyGoalMins, dailyGoalHours);

            if (pastWorkMet) {
                streak += 1;
            } else if (pastHealthShield) {
                // Active Recovery Shield (Freeze/Bridge):
                // Posture habit was fulfilled, so streak is protected from breaking (does not reset to 0),
                // but does NOT falsely increment the work focus counter.
                continue;
            } else {
                // Day was neither worked nor shielded: streak chain ends
                break;
            }
        }

        return streak;
    }

    function calculateHealthStreakDays() {
        const goalBreaks = userData.healthDailyGoalBreaks || 4;
        const dailyGoalHours = parseFloat(userData.dailyGoalHours) || 1;
        const dailyGoalMins = dailyGoalHours * 60;
        let streak = 0;

        // Check Today: ONLY increment if posture stand goal is actually reached today
        const todayBreaks = userData.healthCompletedBreaksToday || 0;
        const todayStandsMet = (todayBreaks >= goalBreaks);

        if (todayStandsMet) {
            streak += 1;
        }

        // Walk backwards starting from Yesterday:
        const todayObj = new Date();
        for (let i = 1; i <= 365; i++) {
            const d = new Date();
            d.setDate(todayObj.getDate() - i);
            const dateStr = getLocalDateStr(d);

            const dayStats = (userData.healthStatsByDate && userData.healthStatsByDate[dateStr])
                ? userData.healthStatsByDate[dateStr]
                : null;
            const pastBreaks = (dayStats && typeof dayStats.breaks === 'number') ? dayStats.breaks : 0;
            const pastStandsMet = (pastBreaks >= goalBreaks);

            let pastMins = (userData.completedMinutesByDate && userData.completedMinutesByDate[dateStr])
                ? userData.completedMinutesByDate[dateStr]
                : 0;

            if (pastMins === 0 && userData.appStatsByDate && userData.appStatsByDate[dateStr]) {
                const dayApps = userData.appStatsByDate[dateStr];
                let totalSec = 0;
                for (const exe in dayApps) totalSec += dayApps[exe];
                pastMins = Math.floor(totalSec / 60);
            }

            const pastWorkMet = isGoalMetWithTolerance(pastMins, dailyGoalMins, dailyGoalHours);

            if (pastStandsMet) {
                streak += 1;
            } else if (pastWorkMet) {
                // Mutual Protection (Freeze/Bridge):
                // A busy deep-work day protects posture streak from breaking,
                // without falsely claiming uncompleted stands.
                continue;
            } else {
                break;
            }
        }
        return streak;
    }

    let lastSyncedStats = {
        completedMinutesToday: -1,
        dailyGoalMinutes: -1,
        streakDays: -1,
        yesterdayHours: -1
    };

    function syncStatsToCpp(force = false) {
        const comp = Number(userData.completedMinutesToday) || 0;
        const goal = Math.max(1, (Number(userData.dailyGoalHours) || 1) * 60);
        const streak = parseInt(userData.streakDays || 0);
        const yHrs = parseFloat(userData.yesterdayHours || 0);

        if (!force &&
            lastSyncedStats.completedMinutesToday === comp &&
            lastSyncedStats.dailyGoalMinutes === goal &&
            lastSyncedStats.streakDays === streak &&
            lastSyncedStats.yesterdayHours === yHrs) {
            return;
        }

        lastSyncedStats = {
            completedMinutesToday: comp,
            dailyGoalMinutes: goal,
            streakDays: streak,
            yesterdayHours: yHrs
        };

        sendToCpp({
            action: 'syncStats',
            completedMinutesToday: comp,
            dailyGoalMinutes: goal,
            streakDays: streak,
            yesterdayHours: yHrs
        });
    }

    let lastMidnightRolloverTime = 0;

    function checkAndHandleMidnightRollover() {
        const currentLocalDate = getLocalDateStr();
        todayDateStr = currentLocalDate;

        if (userData.lastDateStr && userData.lastDateStr !== currentLocalDate) {
            console.log(`[FocusGrow] Midnight rollover detected (00:00 local): ${userData.lastDateStr} -> ${currentLocalDate}`);
            lastMidnightRolloverTime = Date.now();
            const prevDayMins = userData.completedMinutesToday || 0;

            // Save previous day's minutes to historical stats
            userData.completedMinutesByDate = userData.completedMinutesByDate || {};
            userData.completedMinutesByDate[userData.lastDateStr] = prevDayMins;

            // Save previous day's health stats
            userData.healthStatsByDate = userData.healthStatsByDate || {};
            userData.healthStatsByDate[userData.lastDateStr] = {
                breaks: userData.healthCompletedBreaksToday || 0
            };

            // Reset today's stats to 0
            userData.completedMinutesToday = 0;
            userData.healthCompletedBreaksToday = 0;
            userData.lastDateStr = currentLocalDate;

            // Seamless Midnight Continuation:
            // Do NOT stop active focus or break sessions!
            // When user focuses across midnight, the active timer continues running smoothly into the new day.
            // Only clean up stale session data if the timer was already idle.
            if (typeof activeState === 'undefined' || activeState === 'idle') {
                if (userData.activeSession) {
                    delete userData.activeSession;
                }
            }

            // Compute Yesterday Hours dynamically from yesterday's actual date
            const yesterdayObj = new Date();
            yesterdayObj.setDate(yesterdayObj.getDate() - 1);
            const yDateStr = getLocalDateStr(yesterdayObj);
            const yMins = (userData.completedMinutesByDate && userData.completedMinutesByDate[yDateStr])
                ? userData.completedMinutesByDate[yDateStr]
                : 0;
            userData.yesterdayHours = (yMins / 60).toFixed(1);

            // Dynamically recalculate streaks
            userData.streakDays = calculateStreakDays();
            userData.healthStreakDays = calculateHealthStreakDays();
            saveUserData();

            // Sync 0 completed minutes to C++ FocusEngine (countdown continues seamlessly)
            syncStatsToCpp(true);

            // Update UI elements in real time
            updateStatsUI();
            if (typeof renderWeeklyChart === 'function') renderWeeklyChart();
            if (typeof renderMonthlyHeatmap === 'function') renderMonthlyHeatmap();
            if (typeof renderHourlyBarChart === 'function') renderHourlyBarChart();
            if (typeof renderActivityBreakdown === 'function') renderActivityBreakdown();
            if (typeof renderRecentSessionsTable === 'function') renderRecentSessionsTable();
            return true;
        }
        return false;
    }

    function loadUserData() {
        try {
            const saved = localStorage.getItem(STORAGE_KEY);
            if (saved) {
                const parsed = JSON.parse(saved);
                userData = { ...userData, ...parsed };

                // Reset/sanitize to new compact PIP dimensions only if not yet set or completely invalid
                if (!userData.pipMigratedV4) {
                    userData.pipMigratedV4 = true;
                    if (!userData.pipWidth || userData.pipWidth > 440 || userData.pipWidth < 150) userData.pipWidth = 195;
                    if (!userData.pipHeight || userData.pipHeight > 550 || userData.pipHeight < 210) userData.pipHeight = 280;
                    saveUserData();
                } else {
                    if (userData.pipWidth && (userData.pipWidth > 440 || userData.pipWidth < 150)) userData.pipWidth = 195;
                    if (userData.pipHeight && (userData.pipHeight > 550 || userData.pipHeight < 210)) userData.pipHeight = 280;
                }
                delete userData.pipVinylWidth;
                delete userData.pipVinylHeight;

                // Migration: Move base64 GIFs to IndexedDB
                migrateGifsToIndexedDB();

                todayDateStr = getLocalDateStr();
                if (!userData.lastDateStr) {
                    userData.lastDateStr = todayDateStr;
                }

                // Check and handle rollover if PC was turned off across midnight
                checkAndHandleMidnightRollover();

                // Compute Yesterday Hours dynamically from yesterday's actual date
                const yesterdayObj = new Date();
                yesterdayObj.setDate(new Date().getDate() - 1);
                const yDateStr = getLocalDateStr(yesterdayObj);
                const yMins = (userData.completedMinutesByDate && userData.completedMinutesByDate[yDateStr])
                    ? userData.completedMinutesByDate[yDateStr]
                    : 0;
                userData.yesterdayHours = (yMins / 60).toFixed(1);

                // Dynamically recalculate streak
                userData.streakDays = calculateStreakDays();
                saveUserData();
            }
        } catch (e) {
            console.error('Error loading local user data:', e);
        }
    }

    async function migrateGifsToIndexedDB() {
        let changed = false;

        // Migrate active custom GIF
        if (userData.customGifData && userData.customGifData.startsWith('data:image')) {
            const id = 'active_' + Date.now();
            await gifDb.save(id, userData.customGifData);
            userData.customGifData = id;
            changed = true;
        }

        // Migrate recent GIFs
        if (userData.recentGifs && userData.recentGifs.length > 0) {
            for (let i = 0; i < userData.recentGifs.length; i++) {
                const gif = userData.recentGifs[i];
                if (gif.data && gif.data.startsWith('data:image')) {
                    const id = gif.id || 'recent_' + Date.now() + '_' + i;
                    await gifDb.save(id, gif.data);
                    gif.data = id;
                    gif.id = id;
                    changed = true;
                }
            }
        }

        if (changed) {
            saveUserData();
            applyGifTheme();
            renderRecentGifs();
        }
    }

    let _saveUserDataTimer = null;
    function saveUserData() {
        if (_saveUserDataTimer) {
            clearTimeout(_saveUserDataTimer);
            _saveUserDataTimer = null;
        }
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(userData));
        } catch (e) {
            console.error('Error saving local user data:', e);
        }
    }

    function saveUserDataDebounced(delayMs = 5000) {
        if (_saveUserDataTimer) return;
        _saveUserDataTimer = setTimeout(() => {
            _saveUserDataTimer = null;
            saveUserData();
        }, delayMs);
    }

    window.addEventListener('beforeunload', () => {
        if (_saveUserDataTimer) saveUserData();
    });
    document.addEventListener('visibilitychange', () => {
        document.body.classList.toggle('window-hidden', document.hidden);
        if (document.hidden && _saveUserDataTimer) {
            saveUserData();
        } else if (!document.hidden && typeof updatePrayerClock === 'function') {
            updatePrayerClock();
        }
    });
    document.body.classList.toggle('window-hidden', document.hidden);

    loadUserData();

    // Migrate or initialize blockedApps
    if (!userData.blockedApps) {
        userData.blockedApps = [];
        saveUserData();
    }

    let selectedMins = userData.selectedMins || 30;
    let selectedPeriodMins = userData.selectedPeriodMins || 25;
    let selectedBreakMins = userData.selectedBreakMins || 5;
    let blockedApps = userData.blockedApps || [];
    let isPaused = false;
    let activeState = 'idle';
    let previousState = 'idle';
    let previousPeriod = 1;
    let notifiedOneMinWarning = false;
    let cachedAppList = [];

    // Custom Notification Sound Loader & Playback
    window._customFocusAudio = null;
    window._customBreakAudio = null;
    window._customWarningAudio = null;

    async function initCustomSounds() {
        try {
            const focusData = await gifDb.get('focus_sound');
            if (focusData) {
                window._customFocusAudio = new Audio(focusData);
            }
            const breakData = await gifDb.get('break_sound');
            if (breakData) {
                window._customBreakAudio = new Audio(breakData);
            }
            const warningData = await gifDb.get('warning_sound');
            if (warningData) {
                window._customWarningAudio = new Audio(warningData);
            }
        } catch (e) {
            console.error('[Sound] Failed to initialize custom sounds:', e);
        }
    }
    initCustomSounds();

    function playNotificationSound(category = 'focus') {
        let soundType;
        if (category === 'break') {
            soundType = userData.breakNotificationSound || 'chime';
        } else if (category === 'warning') {
            soundType = userData.warningNotificationSound || 'reminder';
        } else {
            soundType = userData.focusNotificationSound || 'default';
        }

        if (soundType === 'none') {
            return 'none';
        } else if (soundType === 'custom') {
            let audioObj;
            if (category === 'break') {
                audioObj = window._customBreakAudio;
            } else if (category === 'warning') {
                audioObj = window._customWarningAudio;
            } else {
                audioObj = window._customFocusAudio;
            }
            if (audioObj) {
                audioObj.currentTime = 0;
                audioObj.play().catch(err => console.error('[Sound] Custom sound playback error:', err));
            } else {
                console.warn(`[Sound] Custom ${category} sound selected but no audio file is loaded.`);
            }
            return 'none'; // Return 'none' to Windows Toast so Windows doesn't double-play system chime
        } else {
            return soundType;
        }
    }

    // Notification Helper
    function sendNotification(title, body, category = 'focus') {
        if (!userData.notificationsEnabled) return;

        // Anti-spam: Don't send identical notification within 1 second
        const now = Date.now();
        const notifKey = title + body;
        if (window._lastNotifKey === notifKey && (now - (window._lastNotifTime || 0)) < 1000) {
            return;
        }
        window._lastNotifKey = notifKey;
        window._lastNotifTime = now;

        const soundToPlay = playNotificationSound(category);

        sendToCpp({
            action: 'notify',
            title: title,
            body: body,
            sound: soundToPlay
        });
    }

    // --- Procedural Transition Countdown Sound Engine (3... 2... 1... Transition) ---
    let countdownAudioCtx = null;
    function getCountdownAudioContext() {
        if (!countdownAudioCtx) {
            const AudioContextClass = window.AudioContext || window.webkitAudioContext;
            if (AudioContextClass) {
                countdownAudioCtx = new AudioContextClass();
            }
        }
        if (countdownAudioCtx && countdownAudioCtx.state === 'suspended') {
            countdownAudioCtx.resume().catch(() => {});
        }
        return countdownAudioCtx;
    }

    function playCountdownPip(step = 3) {
        if (userData.countdownSoundEnabled === false) return;
        const ctx = getCountdownAudioContext();
        if (!ctx) return;
        const now = ctx.currentTime;

        const isZen = (userData.countdownSoundStyle !== 'digital');
        const maxDuration = parseInt(userData.countdownDurationSec) || 3;

        if (isZen) {
            // Aesthetic Zen Chime: warm bell with harmonically tuned overtones
            // Harmonically rising progression towards step 1 (G5 783.99Hz)
            const progress = (maxDuration > 1) ? Math.max(0, Math.min(1, (maxDuration - step) / (maxDuration - 1))) : 1;
            const freq = (step === 1) ? 783.99 : (480 + progress * 240);

            const osc = ctx.createOscillator();
            const oscOvertone = ctx.createOscillator();
            const gain = ctx.createGain();
            const gainOvertone = ctx.createGain();

            osc.type = 'sine';
            osc.frequency.setValueAtTime(freq, now);

            oscOvertone.type = 'triangle';
            oscOvertone.frequency.setValueAtTime(freq * 2.756, now); // Metallic chime harmonic

            gain.gain.setValueAtTime(0.24, now);
            gain.gain.exponentialRampToValueAtTime(0.001, now + 0.6);

            gainOvertone.gain.setValueAtTime(0.07, now);
            gainOvertone.gain.exponentialRampToValueAtTime(0.001, now + 0.3);

            osc.connect(gain);
            oscOvertone.connect(gainOvertone);
            gain.connect(ctx.destination);
            gainOvertone.connect(ctx.destination);

            osc.start(now);
            oscOvertone.start(now);
            osc.stop(now + 0.65);
            oscOvertone.stop(now + 0.35);
        } else {
            // Modern Digital Pip (Clean High-Tech Beep)
            const progress = (maxDuration > 1) ? Math.max(0, Math.min(1, (maxDuration - step) / (maxDuration - 1))) : 1;
            const freq = (step === 1) ? 1046.5 : (840 + progress * 160);

            const osc = ctx.createOscillator();
            const gain = ctx.createGain();
            osc.type = 'sine';
            osc.frequency.setValueAtTime(freq, now);

            gain.gain.setValueAtTime(0.28, now);
            gain.gain.exponentialRampToValueAtTime(0.001, now + 0.16);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(now);
            osc.stop(now + 0.18);
        }
    }

    function playCountdownTransitionImpact(toState = 'break') {
        if (userData.countdownSoundEnabled === false) return;
        const ctx = getCountdownAudioContext();
        if (!ctx) return;
        const now = ctx.currentTime;

        const isZen = (userData.countdownSoundStyle !== 'digital');

        if (isZen) {
            // Rich Tibetan Singing Bowl / Warm Genta Chime
            const rootFreq = (toState === 'focus') ? 523.25 : 440.0;
            const harmonics = [1, 2.01, 3.02, 4.18];
            const gains = [0.32, 0.16, 0.08, 0.03];

            harmonics.forEach((mult, i) => {
                const osc = ctx.createOscillator();
                const gain = ctx.createGain();
                osc.type = 'sine';
                osc.frequency.setValueAtTime(rootFreq * mult, now);

                gain.gain.setValueAtTime(gains[i], now);
                gain.gain.exponentialRampToValueAtTime(0.0005, now + 2.2);

                osc.connect(gain);
                gain.connect(ctx.destination);

                osc.start(now);
                osc.stop(now + 2.3);
            });
        } else {
            // Punchy Modern Impact / Transition Chime
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();
            osc.type = 'sine';
            osc.frequency.setValueAtTime(160, now);
            osc.frequency.exponentialRampToValueAtTime(45, now + 0.35);

            gain.gain.setValueAtTime(0.4, now);
            gain.gain.exponentialRampToValueAtTime(0.001, now + 0.7);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(now);
            osc.stop(now + 0.75);
        }
    }

    // --- YTMPX WebSocket Realtime Sync ---
    let ytmpxSocket = null;
    let isUsingExtension = false; // Flag to prioritize extension data over browser tab titles
    let ytTrackData = {
        title: '',
        author: '',
        image: '',
        isPlaying: false,
        source: '', // 'tauri' | 'browser'
        exeName: ''
    };

    let trackClearTimer = null;

    function clearTrackDataGracefully(delayMs = 2500) {
        if (trackClearTimer) return;
        trackClearTimer = setTimeout(() => {
            trackClearTimer = null;
            console.log('[YTMusic] Grace period expired. Clearing track info.');
            ytTrackData.title = '';
            ytTrackData.author = '';
            ytTrackData.image = '';
            ytTrackData.isPlaying = false;
            ytTrackData.source = '';
            ytTrackData.exeName = '';
            lastFetchedTrackKey = '';
            updateYtMusicUI();
        }, delayMs);
    }

    function cancelTrackDataClear() {
        if (trackClearTimer) {
            clearTimeout(trackClearTimer);
            trackClearTimer = null;
        }
    }

    function connectYtmpxWebSocket() {
        if (ytmpxSocket && (ytmpxSocket.readyState === WebSocket.OPEN || ytmpxSocket.readyState === WebSocket.CONNECTING)) {
            return;
        }

        try {
            ytmpxSocket = new WebSocket('ws://127.0.0.1:8765');

            ytmpxSocket.onopen = () => {
                console.log('[YTMPX] Connected to ws://127.0.0.1:8765');
                isUsingExtension = true;
            };

            ytmpxSocket.onmessage = (evt) => {
                try {
                    const data = JSON.parse(evt.data);
                    if (data) {
                        // Ignore command echoes broadcasted by the server (e.g. volumeUp, volumeDown, playPause)
                        if (data.command) {
                            return;
                        }

                        isUsingExtension = true; // Confirmed working extension

                        // Identify source (Tauri vs Browser)
                        const isTauri = (data.source === 'ytm-tauri');

                        if (isTauri) {
                            ytTrackData.source = 'tauri';
                            ytTrackData.exeName = 'ytm-tauri.exe';
                        } else {
                            // Any non-tauri payload is from browser (either Chrome YTMPX extension or Python helper)
                            ytTrackData.source = 'browser';
                            if (data.exeName) {
                                ytTrackData.exeName = data.exeName;
                            } else {
                                const browserApp = (cachedAppList || []).find(a => {
                                    const exe = (a.exeName || '').toLowerCase();
                                    return exe === 'chrome.exe' || exe === 'msedge.exe' || exe === 'brave.exe' || exe === 'firefox.exe' || exe === 'opera.exe' || exe === 'vivaldi.exe';
                                });
                                ytTrackData.exeName = browserApp ? browserApp.exeName : 'chrome.exe';
                            }
                        }

                        if (data.event === 'stop' || !data.metadata || (!data.metadata.title && !data.metadata.author)) {
                            console.log('[YTMPX] Transient stop/empty track event. Scheduling graceful clear...');
                            ytTrackData.isPlaying = false;
                            updateYtMusicUI();
                            clearTrackDataGracefully(2500);
                        } else if (data.metadata && (data.metadata.title || data.metadata.author)) {
                            cancelTrackDataClear(); // New track info confirmed! Cancel any pending clear!

                            const newTitle = data.metadata.title || ytTrackData.title;
                            const newAuthor = data.metadata.author || ytTrackData.author;
                            const trackChanged = (newTitle !== ytTrackData.title || newAuthor !== ytTrackData.author);

                            if (trackChanged) {
                                cachedTrackImageBase64 = "";
                                cachedOriginalImageUrl = "";
                                lastSentImageUrl = "";
                            }

                            ytTrackData.title = newTitle;
                            ytTrackData.author = newAuthor;

                            // Always update image if data has an image and it's new or missing
                            let imageChanged = false;
                            if (data.metadata && data.metadata.image) {
                                if (!ytTrackData.image || ytTrackData.image !== data.metadata.image || trackChanged) {
                                    ytTrackData.image = data.metadata.image;
                                    imageChanged = true;
                                }
                            }

                            let playStateChanged = false;
                            if (data.event === 'track' || data.event === 'resume') {
                                if (!ytTrackData.isPlaying) {
                                    ytTrackData.isPlaying = true;
                                    playStateChanged = true;
                                }
                            } else if (data.event === 'pause') {
                                // Ignore transient pause event if user just adjusted volume
                                if (Date.now() - lastVolumeScrollTime > 1500) {
                                    if (ytTrackData.isPlaying) {
                                        ytTrackData.isPlaying = false;
                                        playStateChanged = true;
                                    }
                                }
                            }

                            // Only trigger full UI and C++ vinyl update if metadata or play state actually changed
                            if (trackChanged || imageChanged || playStateChanged) {
                                updateYtMusicUI();
                            }
                        }
                    }
                } catch (err) {
                    console.error('[YTMPX] Error parsing message:', err);
                }
            };

            ytmpxSocket.onclose = () => {
                console.log('[YTMPX] Extension websocket closed');
                isUsingExtension = false;
                if (ytTrackData.source === 'tauri') {
                    // Tauri app was exited/closed completely -> retract vinyl disc
                    clearTrackDataGracefully(800);
                } else {
                    detectYtMusicFromBrowserTabs(cachedAppList);
                }
                setTimeout(connectYtmpxWebSocket, 3000);
            };

            ytmpxSocket.onerror = () => {
                isUsingExtension = false;
                if (ytTrackData.source === 'tauri') {
                    clearTrackDataGracefully(800);
                }
                ytmpxSocket.close();
            };
        } catch (e) {
            isUsingExtension = false;
            setTimeout(connectYtmpxWebSocket, 3000);
        }
    }

    connectYtmpxWebSocket();

    let lastFetchedTrackKey = '';

    function fetchAlbumArtFromiTunes(title, author) {
        if (!title || title.toLowerCase() === 'youtube music' || title.toLowerCase() === 'yt music') return;
        const trackKey = `${title}-${author}`;
        if (lastFetchedTrackKey === trackKey) return;
        lastFetchedTrackKey = trackKey;

        // Clean parentheticals like (feat. Aizawa), [MV], etc.
        const cleanTitle = title.replace(/\([^\)]+\)/g, '').replace(/\[[^\]]+\]/g, '').split(/\s*[\-\|]\s*/)[0].trim();
        const cleanAuthor = (author && author !== 'YouTube Music')
            ? author.replace(/\([^\)]+\)/g, '').replace(/\[[^\]]+\]/g, '').split(/\s*[\-\|]\s*/)[0].trim()
            : '';

        const searchQueries = [
            `${cleanTitle} ${cleanAuthor}`,
            cleanTitle,
            cleanAuthor
        ].filter(q => q && q.length > 0);

        function tryNextQuery(index) {
            if (index >= searchQueries.length) return;
            // Skip fetch if not on a server (CORS issues in some WebView2 environments)
            if (window.location.protocol === 'file:') {
                console.warn('[iTunes] Skip fetch on file:// to avoid CORS. Use WebSocket metadata if available.');
                return;
            }
            const q = encodeURIComponent(searchQueries[index]);
            fetch(`https://itunes.apple.com/search?term=${q}&media=music&limit=1`)
                .then(res => res.json())
                .then(data => {
                    if (data && data.results && data.results.length > 0 && data.results[0].artworkUrl100) {
                        const hdCover = data.results[0].artworkUrl100.replace('100x100bb', '600x600bb');
                        ytTrackData.image = hdCover;
                        updateYtMusicUI();
                    } else {
                        tryNextQuery(index + 1);
                    }
                })
                .catch(err => {
                    console.error('[iTunes] Fetch error:', err);
                    tryNextQuery(index + 1);
                });
        }

        tryNextQuery(0);
    }

    function detectYtMusicFromBrowserTabs(apps) {
        if (!apps || !Array.isArray(apps)) return;

        // If Tauri is actively connected via WebSocket, NEVER clear track data from window enumeration!
        // Tauri runs in the system tray when minimized, so its window won't be visible in C++ EnumWindows.
        if (isUsingExtension && ytTrackData.source === 'tauri') {
            cancelTrackDataClear();
            return;
        }

        // If WebSocket is open and connected to any extension/Tauri, trust WebSocket over window scans
        if (ytmpxSocket && ytmpxSocket.readyState === WebSocket.OPEN && (ytTrackData.source === 'tauri' || isUsingExtension)) {
            cancelTrackDataClear();
            return;
        }

        // Find all potential YouTube Music/YouTube tabs or app processes (exclude ytmpx helper)
        const ytApps = apps.filter(app => {
            if (!app.title && !app.exeName) return false;
            const t = (app.title || '').toLowerCase();
            const e = (app.exeName || '').toLowerCase();
            if (e.includes('ytmpx')) return false;
            return t.includes('youtube music') || t.includes('yt music') || e === 'ytm-tauri.exe' || e === 'youtube music.exe' || e === 'ytmdesktop.exe';
        });

        // 1. If NO YouTube Music tab/app exists on the system at all:
        if (ytApps.length === 0) {
            // Guard: If WebSocket is connected to Tauri or extension, DO NOT clear!
            if (isUsingExtension || (ytmpxSocket && ytmpxSocket.readyState === WebSocket.OPEN)) {
                return;
            }
            if (ytTrackData.title || ytTrackData.author || ytTrackData.isPlaying) {
                clearTrackDataGracefully(1500);
            }
            return;
        }

        // 2. If YouTube Music IS running:
        cancelTrackDataClear(); // Active YT Music app found! Cancel clear!

        if (isUsingExtension && ytTrackData.source === 'tauri') {
            return; // Tauri is actively providing track info, so trust Tauri metadata
        }

        // 3. Fallback: Parse track from browser tab title
        const ytApp = ytApps.find(app => app.title.includes(' - ') || app.title.includes(' | ')) || ytApps[0];

        let rawTitle = ytApp.title || '';
        let cleaned = rawTitle.replace(/\s*[\-\|]\s*YouTube Music/gi, '')
            .replace(/^YouTube Music\s*[\-\|]\s*/gi, '')
            .replace(/\s*[\-\|]\s*YouTube/gi, '')
            .trim();

        const isGeneric = !cleaned ||
            cleaned.toLowerCase() === 'youtube music' ||
            cleaned.toLowerCase() === 'yt music' ||
            cleaned.toLowerCase() === 'youtube';

        if (!isGeneric) {
            const parts = cleaned.split(/\s*[\-\|]\s*/);
            let newTitle = parts.length >= 2 ? parts[0].trim() : cleaned;
            let newAuthor = parts.length >= 2 ? parts[1].trim() : 'YouTube Music';

            const exeLower = (ytApp.exeName || '').toLowerCase();
            const isNative = exeLower === 'ytm-tauri.exe' || exeLower === 'youtube music.exe' || exeLower === 'ytmdesktop.exe';
            const newSource = isNative ? 'tauri' : 'browser';
            const newExe = ytApp.exeName || (isNative ? 'ytm-tauri.exe' : 'chrome.exe');

            if (ytTrackData.title !== newTitle || ytTrackData.author !== newAuthor || !ytTrackData.isPlaying || ytTrackData.source !== newSource) {
                const trackChanged = ytTrackData.title !== newTitle || ytTrackData.author !== newAuthor;
                ytTrackData.title = newTitle;
                ytTrackData.author = newAuthor;
                ytTrackData.isPlaying = true;
                ytTrackData.source = newSource;
                ytTrackData.exeName = newExe;

                console.log(`[TabDetect] Track detected (${newSource} - ${newExe}): ${newTitle} — ${newAuthor}`);

                if (trackChanged || !ytTrackData.image) {
                    fetchAlbumArtFromiTunes(ytTrackData.title, ytTrackData.author);
                }
                updateYtMusicUI();
            }
        }
    }

    function updateYtMusicUI() {
        const textSetup = document.getElementById('ticker-text-setup');
        const textTimer = document.getElementById('ticker-text-timer');
        const tickerSetup = document.getElementById('ytmusic-ticker-setup');
        const tickerTimer = document.getElementById('ytmusic-ticker-timer');

        const vinylCoverImg = document.getElementById('vinyl-cover-img');
        const cardVinylCoverImg = document.getElementById('card-vinyl-cover-img');
        const vinylDisc = document.getElementById('vinyl-disc');
        const cardVinylDisc = document.getElementById('card-vinyl-disc');
        const cardVinylContainer = document.getElementById('card-vinyl-container');

        const isYtMode = (userData.ambientMode === 'ytmusic');
        const hasTrack = !!(ytTrackData.title || ytTrackData.author);

        const trackString = (hasTrack)
            ? `${ytTrackData.title} — ${ytTrackData.author}`
            : 'Not playing — YT Music';

        if (textSetup) {
            textSetup.textContent = trackString;
            textSetup.title = hasTrack ? `${trackString}\nClick to toggle Music player • Scroll to adjust Volume` : 'Click to toggle Music player';
        }
        if (textTimer) {
            textTimer.textContent = trackString;
            textTimer.title = hasTrack ? `${trackString}\nClick to toggle Music player • Scroll to adjust Volume` : 'Click to toggle Music player';
        }

        // Automatically HIDE the YT Music ticker & control buttons when YT Music is not open/playing
        const showTicker = hasTrack;
        if (tickerSetup) {
            tickerSetup.style.setProperty('display', showTicker ? 'flex' : 'none', 'important');
            tickerSetup.classList.toggle('hidden', !showTicker);
        }
        if (tickerTimer) {
            tickerTimer.style.setProperty('display', showTicker ? 'flex' : 'none', 'important');
            tickerTimer.classList.toggle('hidden', !showTicker);
        }

        // ONLY SHOW vinyl container if in YT Music ambient mode AND a track is detected
        if (cardVinylContainer) {
            if (isYtMode && hasTrack) {
                cardVinylContainer.style.display = 'block';
                cardVinylContainer.style.opacity = '1';
                cardVinylContainer.style.visibility = 'visible';
            } else if (!isYtMode) {
                cardVinylContainer.style.display = 'none';
            }
        }

        const defaultCover = defaultVinylSvg;
        // Keep the image if it exists
        const coverSrc = (ytTrackData.image) ? ytTrackData.image : defaultCover;
        syncVinylCoverWithCpp(coverSrc);

        // Sync vinyl animation state with actual playback
        const isActuallyPlaying = ytTrackData.isPlaying;
        if (vinylCoverImg) {
            vinylCoverImg.src = coverSrc;
            vinylCoverImg.classList.toggle('paused', !isActuallyPlaying);
        }
        if (cardVinylCoverImg) {
            cardVinylCoverImg.src = coverSrc;
            cardVinylCoverImg.classList.toggle('paused', !isActuallyPlaying);
        }

        // Sync media control icons with isActuallyPlaying
        document.querySelectorAll('.btn-yt-playpause').forEach(btn => {
            const pausePath = btn.querySelector('.icon-pause-path');
            const playPath = btn.querySelector('.icon-play-path');
            if (pausePath && playPath) {
                pausePath.style.display = !isActuallyPlaying ? 'none' : 'block';
                playPath.style.display = !isActuallyPlaying ? 'block' : 'none';
            }
        });

        // If no image, keep trying to fetch it
        if (!ytTrackData.image && ytTrackData.title) {
            fetchAlbumArtFromiTunes(ytTrackData.title, ytTrackData.author);
        }

        // Dynamic YT Music Album Accent Color Extraction Mode
        const isDynamicAlbum = !!(userData.dynamicAlbumEnabled || userData.accentMode === 'ytmusic_dynamic');
        if (isDynamicAlbum) {
            // Keep dynamic color if we have a track, even if paused
            if (hasTrack && coverSrc && coverSrc !== defaultCover) {
                extractDominantColor(coverSrc, (dynamicHex) => {
                    applyAccentTheme(dynamicHex);
                });
            } else {
                // When no music is playing, seamlessly fallback to user's selected base accent color (e.g. Orange)!
                applyAccentTheme(userData.accentColor || '#60cdff');
            }
        } else {
            applyAccentTheme(userData.accentColor || '#60cdff');
        }

        // Update PIP Peeking Vinyl Disc
        updatePipPeekingVinyl();
    }

    let cachedTrackImageBase64 = "";
    let cachedOriginalImageUrl = "";

    function syncVinylCoverWithCpp(imageUrl) {
        if (!imageUrl) {
            cachedTrackImageBase64 = "";
            cachedOriginalImageUrl = "";
            updatePipPeekingVinyl();
            return;
        }
        if (imageUrl === cachedOriginalImageUrl && cachedTrackImageBase64) {
            return;
        }
        cachedOriginalImageUrl = imageUrl;

        // If it's already a data URL, use it immediately
        if (imageUrl.startsWith('data:image/')) {
            cachedTrackImageBase64 = imageUrl;
            updatePipPeekingVinyl();
            return;
        }

        // Bridge the browser's already loaded image to C++ while strictly PRESERVING original aspect ratio (16:9, 1:1, etc.)
        const img = new Image();
        img.crossOrigin = "anonymous";
        img.onload = () => {
            try {
                const nw = img.naturalWidth || img.width || 320;
                const nh = img.naturalHeight || img.height || 320;
                const maxDim = 480;
                let targetW = nw;
                let targetH = nh;
                if (nw > maxDim || nh > maxDim) {
                    if (nw >= nh) {
                        targetW = maxDim;
                        targetH = Math.max(1, Math.round((nh / nw) * maxDim));
                    } else {
                        targetH = maxDim;
                        targetW = Math.max(1, Math.round((nw / nh) * maxDim));
                    }
                }
                const canvas = document.createElement('canvas');
                canvas.width = targetW;
                canvas.height = targetH;
                const ctx = canvas.getContext('2d');
                ctx.drawImage(img, 0, 0, targetW, targetH);
                cachedTrackImageBase64 = canvas.toDataURL('image/png');
                updatePipPeekingVinyl();
            } catch (e) {
                // If canvas tainted due to CORS, fallback to original URL
                cachedTrackImageBase64 = imageUrl;
                updatePipPeekingVinyl();
            }
        };
        img.onerror = () => {
            cachedTrackImageBase64 = imageUrl;
            updatePipPeekingVinyl();
        };
        img.src = imageUrl;
    }

    let lastPeekingTrackKey = "";
    let lastSentImageUrl = "";
    let isPeekingTransitioning = false;

    function updatePipPeekingVinyl() {
        if (isPipMorphing ||
            document.body.classList.contains('pip-morphing-out') ||
            document.body.classList.contains('pip-morphing-in')) {
            sendToCpp({
                action: 'updatePipVinylOverlay',
                visible: false
            });
            return;
        }

        const isPip = document.body.classList.contains('pip-mode');
        const isEnabled = !!userData.pipPeekingVinylEnabled;
        const side = userData.pipPeekingVinylSide || 'left';
        const hasTrack = !!(ytTrackData.title || ytTrackData.author);
        const isPlaying = !!ytTrackData.isPlaying;

        const currentTrackKey = `${ytTrackData.title}::${ytTrackData.author}`;
        if (currentTrackKey !== lastPeekingTrackKey) {
            lastPeekingTrackKey = currentTrackKey;
            lastSentImageUrl = "";
            cachedTrackImageBase64 = "";
            cachedOriginalImageUrl = "";
        }

        let effectiveImageUrl = cachedTrackImageBase64 || ytTrackData.image || '';
        // If we already sent a valid image URL for this exact song, stick to it to prevent C++ vinyl retraction bounces
        if (lastSentImageUrl && currentTrackKey && currentTrackKey === lastPeekingTrackKey) {
            effectiveImageUrl = lastSentImageUrl;
        } else if (effectiveImageUrl) {
            lastSentImageUrl = effectiveImageUrl;
        }

        // Show peeking vinyl whenever enabled and track is active
        const shouldShowPeeking = isEnabled && hasTrack;

        // Sync with Native C++ Layered Vinyl Overlay
        sendToCpp({
            action: 'updatePipVinylOverlay',
            visible: shouldShowPeeking,
            side: side,
            isPlaying: isPlaying,
            imageUrl: effectiveImageUrl,
            title: ytTrackData.title || '',
            exeName: ytTrackData.exeName || '',
            source: ytTrackData.source || '',
            speedSec: userData.vinylSpeed || 6
        });
    }

    // Dynamic Accent Color Manager & Realtime CSS Variable Applicator
    function applyAccentTheme(hexColor) {
        if (!hexColor) return;

        // --- Accessibility: Luminance Check ---
        const getLuminance = (hex) => {
            const rgb = hex.match(/[A-Za-z0-9]{2}/g).map(v => parseInt(v, 16) / 255);
            const a = rgb.map(v => v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4));
            return a[0] * 0.2126 + a[1] * 0.7152 + a[2] * 0.0722;
        };

        let finalColor = hexColor;
        const lum = getLuminance(hexColor);
        if (lum < 0.12) { // Too dark for dark theme - brighten it
            finalColor = '#80d8ff';
        }

        document.documentElement.style.setProperty('--accent-blue', finalColor);
        document.documentElement.style.setProperty('--accent-blue-hover', finalColor);

        // Dynamic Heatmap shades from Accent Color
        document.documentElement.style.setProperty('--heatmap-lvl-1', `color-mix(in srgb, ${finalColor} 25%, #161b22)`);
        document.documentElement.style.setProperty('--heatmap-lvl-2', `color-mix(in srgb, ${finalColor} 50%, #161b22)`);
        document.documentElement.style.setProperty('--heatmap-lvl-3', `color-mix(in srgb, ${finalColor} 75%, #161b22)`);
        document.documentElement.style.setProperty('--heatmap-lvl-4', finalColor);
        document.documentElement.style.setProperty('--heatmap-lvl-3-glow', `color-mix(in srgb, ${finalColor} 40%, transparent)`);
        document.documentElement.style.setProperty('--heatmap-lvl-4-glow', `color-mix(in srgb, ${finalColor} 60%, transparent)`);

        // Dynamic 24-Hour Productivity Flow SVG Wave Gradients & Peak Marker
        const waveFillGrad = document.getElementById('waveFillGrad');
        if (waveFillGrad) {
            waveFillGrad.innerHTML = `
                <stop offset="0%" stop-color="${finalColor}" stop-opacity="0.38"/>
                <stop offset="60%" stop-color="${finalColor}" stop-opacity="0.10"/>
                <stop offset="100%" stop-color="${finalColor}" stop-opacity="0.0"/>
            `;
        }
        const waveStrokeGrad = document.getElementById('waveStrokeGrad');
        if (waveStrokeGrad) {
            waveStrokeGrad.innerHTML = `
                <stop offset="0%" stop-color="${finalColor}"/>
                <stop offset="65%" stop-color="${finalColor}"/>
                <stop offset="100%" stop-color="${finalColor}"/>
            `;
        }
        const markerPulse = document.getElementById('peak-marker-pulse');
        if (markerPulse) markerPulse.setAttribute('fill', `color-mix(in srgb, ${finalColor} 35%, transparent)`);
        const markerDot = document.getElementById('peak-marker-dot');
        if (markerDot) markerDot.setAttribute('fill', finalColor);

        const picker = document.getElementById('accent-color-picker');
        if (picker && finalColor.startsWith('#')) picker.value = finalColor;

        const isDynamic = !!(userData.dynamicAlbumEnabled || userData.accentMode === 'ytmusic_dynamic');

        // Sync active state of chips with base accent color
        const currentAccent = userData.accentColor || '#60cdff';
        const isPreset = ['#60cdff', '#ff2a70', '#00e676', '#a855f7', '#f59e0b'].includes(currentAccent);
        document.querySelectorAll('.accent-chip').forEach(chip => {
            const val = chip.getAttribute('data-accent');
            chip.classList.toggle('active', val === currentAccent && !isDynamic);
        });

        const customColorIndicator = document.getElementById('custom-color-indicator');
        const customAccentChip = document.getElementById('custom-accent-chip');
        if (customColorIndicator) customColorIndicator.style.background = currentAccent;
        if (customAccentChip) {
            customAccentChip.style.borderLeft = `4px solid ${currentAccent}`;
            customAccentChip.classList.toggle('active', (!isPreset || userData.accentMode === 'custom') && !isDynamic);
        }

        const chkDynamicAlbum = document.getElementById('chk-dynamic-album');
        if (chkDynamicAlbum) {
            chkDynamicAlbum.checked = isDynamic;
        }

        const baseAccentControls = document.getElementById('base-accent-controls');
        if (baseAccentControls) {
            baseAccentControls.style.opacity = isDynamic ? '0.45' : '1';
            baseAccentControls.style.pointerEvents = isDynamic ? 'none' : 'auto';
        }

        const statusBadge = document.getElementById('dynamic-album-status-badge');
        if (statusBadge) {
            statusBadge.style.display = isDynamic ? 'inline-block' : 'none';
        }

        // Show / Hide Dynamic Complementary Contrast Toggle Row in Options
        const rowDynamicContrast = document.getElementById('row-dynamic-contrast');
        if (rowDynamicContrast) {
            rowDynamicContrast.style.display = isDynamic ? 'flex' : 'none';
        }
    }

    // Color Utility: RGB to HSL conversion
    function rgbToHsl(r, g, b) {
        r /= 255; g /= 255; b /= 255;
        const max = Math.max(r, g, b), min = Math.min(r, g, b);
        let h = 0, s = 0, l = (max + min) / 2;

        if (max !== min) {
            const d = max - min;
            s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
            switch (max) {
                case r: h = (g - b) / d + (g < b ? 6 : 0); break;
                case g: h = (b - r) / d + 2; break;
                case b: h = (r - g) / d + 4; break;
            }
            h /= 6;
        }
        return [h, s, l];
    }

    // Color Utility: HSL to Hex conversion
    function hslToHex(h, s, l) {
        let r, g, b;
        if (s === 0) {
            r = g = b = l;
        } else {
            const hue2rgb = (p, q, t) => {
                if (t < 0) t += 1;
                if (t > 1) t -= 1;
                if (t < 1 / 6) return p + (q - p) * 6 * t;
                if (t < 1 / 2) return q;
                if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
                return p;
            };
            const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
            const p = 2 * l - q;
            r = hue2rgb(p, q, h + 1 / 3);
            g = hue2rgb(p, q, h);
            b = hue2rgb(p, q, h - 1 / 3);
        }
        const toHex = x => {
            const hex = Math.round(x * 255).toString(16);
            return hex.length === 1 ? '0' + hex : hex;
        };
        return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
    }

    // Calculate High-Contrast Complementary Color (Opposite Hue on Color Wheel)
    function getComplementaryColor(r, g, b) {
        let [h, s, l] = rgbToHsl(r, g, b);
        // Rotate hue by 180 degrees (0.5 in normalized 0-1 range)
        h = (h + 0.5) % 1.0;
        // Boost saturation & constrain lightness for max readability against album backdrop
        s = Math.max(s, 0.75);
        l = Math.max(0.55, Math.min(0.72, l));
        return hslToHex(h, s, l);
    }

    // Extract Vibrant Dominant (or Complementary) Color from Album Cover Canvas
    function extractDominantColor(imageUrl, callback) {
        if (!imageUrl) return;
        const img = new Image();
        img.crossOrigin = "Anonymous";
        img.onload = () => {
            try {
                const canvas = document.createElement('canvas');
                canvas.width = 32;
                canvas.height = 32;
                const ctx = canvas.getContext('2d');
                ctx.drawImage(img, 0, 0, 32, 32);
                const data = ctx.getImageData(0, 0, 32, 32).data;

                let rSum = 0, gSum = 0, bSum = 0, colorCount = 0;
                let totalCount = 0;

                for (let i = 0; i < data.length; i += 4) {
                    const r = data[i], g = data[i + 1], b = data[i + 2];
                    const max = Math.max(r, g, b);
                    const min = Math.min(r, g, b);
                    const saturation = max === 0 ? 0 : (max - min) / max;

                    if (max > 20 && max < 250) {
                        totalCount++;
                        // Only count as colorful pixel if saturation is clearly visible (> 0.22)
                        if (saturation > 0.22) {
                            rSum += r; gSum += g; bSum += b;
                            colorCount++;
                        }
                    }
                }

                // If less than 10% of pixels have rich color, it is a Monochrome/Grayscale artwork!
                if (colorCount < Math.max(12, totalCount * 0.10)) {
                    // Monochrome / Black & White / Grayscale -> Clean Sleek Silver/Ice-White
                    console.log('[Color] Monochrome/Grayscale album art detected -> applying #f1f5f9');
                    callback('#f1f5f9');
                    return;
                }

                const r = Math.round(rSum / colorCount);
                const g = Math.round(gSum / colorCount);
                const b = Math.round(bSum / colorCount);

                let [h, s, l] = rgbToHsl(r, g, b);

                if (userData.useComplementaryColor !== false && s > 0.35) {
                    // High contrast complementary for vivid colorful covers
                    h = (h + 0.5) % 1.0;
                    s = Math.max(s, 0.70);
                    l = Math.max(0.55, Math.min(0.72, l));
                    callback(hslToHex(h, s, l));
                } else {
                    // Dominant colorful tone with enhanced visibility
                    s = Math.max(s, 0.65);
                    l = Math.max(0.55, Math.min(0.75, l));
                    callback(hslToHex(h, s, l));
                }
            } catch (e) {
                console.error('[Color] Error extracting dominant color:', e);
                callback(userData.accentColor || '#f1f5f9');
            }
        };
        img.onerror = () => callback(userData.accentColor || '#60cdff');
        img.src = imageUrl;
    }

    // Synchronize Full-Panel Vinyl Center Point to Exact Center of Timer Clock Ring Dial
    function syncVinylCenterPosition() {
        const focusCard = document.getElementById('focus-card');
        const gaugeContainer = document.querySelector('.gauge-container');
        const cardVinylDisc = document.querySelector('.card-vinyl-disc');

        if (!focusCard || !cardVinylDisc) return;

        let centerY = 208;
        let centerX = focusCard.offsetWidth / 2;

        if (gaugeContainer) {
            const cardRect = focusCard.getBoundingClientRect();
            const gaugeRect = gaugeContainer.getBoundingClientRect();
            if (gaugeRect.height > 0 && cardRect.height > 0) {
                centerY = (gaugeRect.top + gaugeRect.height / 2) - cardRect.top;
                centerX = (gaugeRect.left + gaugeRect.width / 2) - cardRect.left;
            }
        }

        cardVinylDisc.style.top = `${centerY.toFixed(1)}px`;
        cardVinylDisc.style.left = `${centerX.toFixed(1)}px`;
    }
    window.addEventListener('resize', () => {
        syncVinylCenterPosition();

        // Save PIP size when user resizes window manually
        if (document.body.classList.contains('pip-mode') && !isInitializingPip) {
            const w = document.documentElement.clientWidth;
            const h = document.documentElement.clientHeight;

            // Strictly guard within valid PIP dimensions (prevents capturing dashboard window size e.g. 857x677)
            if (w >= 150 && w <= 440 && h >= 210 && h <= 550) {
                userData.pipWidth = w;
                userData.pipHeight = h;
                saveUserData();
                console.log(`[PIP] Manual Resize Saved: ${w}x${h}`);
            }
            updatePipPeekingVinyl();
        }
    });

    // Use ResizeObserver for more reliable centering during layout changes (like PIP toggle)
    const gaugeContainerForObserve = document.querySelector('.gauge-container');
    if (gaugeContainerForObserve) {
        const ro = new ResizeObserver(() => {
            requestAnimationFrame(syncVinylCenterPosition);
        });
        ro.observe(gaugeContainerForObserve);
    }

    // Realtime GIF Theme & Display Mode Renderer
    function applyTimerTheme() {
        const gaugeSvg = document.querySelector('.gauge-svg');
        if (!gaugeSvg) return;

        // Apply Stealth Mode class to body for global text hiding
        document.body.classList.toggle('stealth-active', !!userData.isStealthMode);
        document.body.classList.toggle('stealth-hover-peek', userData.stealthHoverPeek !== false);

        // Clean up classes
        gaugeSvg.classList.remove('theme-hourglass', 'theme-wave', 'theme-blocks', 'theme-dots', 'theme-orbit');

        // Hide all layers
        document.querySelectorAll('.theme-layer').forEach(layer => layer.style.display = 'none');
        const orbitParticle = document.getElementById('orbit-particle');
        if (orbitParticle) orbitParticle.style.display = 'none';

        const theme = userData.timerTheme || 'classic';
        if (theme !== 'classic') {
            gaugeSvg.classList.add(`theme-${theme}`);
        }

        // Toggle 5s hourglass rotation
        gaugeSvg.classList.toggle('no-hourglass-rotate', !userData.hourglassAutoRotate);

        const rowHourglassRotate = document.getElementById('row-hourglass-rotate');
        if (rowHourglassRotate) {
            rowHourglassRotate.style.display = (theme === 'hourglass') ? 'flex' : 'none';
        }

        if (theme === 'hourglass') {
            const isFlowing = activeState !== 'idle' && !isPaused;
            updateSandStreamState(isFlowing);
        } else {
            updateSandStreamState(false);
        }

        if (theme === 'wave') {
            const layer = document.querySelector('.layer-wave');
            if (layer) layer.style.display = 'block';
        } else if (theme === 'hourglass') {
            const layer = document.querySelector('.layer-hourglass');
            if (layer) layer.style.display = 'block';
        } else if (theme === 'orbit') {
            if (orbitParticle) orbitParticle.style.display = 'block';
        }

        // Update chips
        document.querySelectorAll('.ring-theme-chip').forEach(chip => {
            chip.classList.toggle('active', chip.getAttribute('data-theme') === theme);
        });
    }

    function updateSandStreamState(isFlowing) {
        const sandStream = document.getElementById('sand-stream');
        if (!sandStream) return;

        if (isFlowing) {
            sandStream.style.display = 'block';
            sandStream.classList.remove('stream-draining', 'stream-starting');
            sandStream.classList.add('stream-active');
        } else {
            sandStream.style.display = 'none';
            sandStream.classList.remove('stream-starting', 'stream-active', 'stream-draining');
        }
    }

    // Timer Ring Theme Switcher
    document.querySelectorAll('.ring-theme-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            userData.timerTheme = chip.getAttribute('data-theme') || 'classic';
            saveUserData();
            applyTimerTheme();
            // Trigger a refresh of the current progress
            if (lastRatio !== undefined) updateVisualProgress(lastRatio);
        });
    });

    // Stealth Mode Click Listener (Toggle visibility of digits)
    document.getElementById('gauge-clickable-area')?.addEventListener('click', () => {
        userData.isStealthMode = !userData.isStealthMode;
        saveUserData();
        document.body.classList.toggle('stealth-active', !!userData.isStealthMode);
        console.log(`[Stealth] Timer digits hidden: ${userData.isStealthMode}`);
    });

    let lastRatio = 1.0;

    function updateVisualProgress(ratio) {
        lastRatio = ratio;
        const theme = userData.timerTheme || 'classic';
        const progressRatio = Math.max(0, Math.min(1.0, 1 - ratio));

        // Update standard circle progress
        if (gaugeProgressBar) {
            const offset = 477.5 * (1 - ratio);
            gaugeProgressBar.style.strokeDashoffset = offset;
        }

        // Theme Specific Logic
        if (theme === 'wave') {
            const levelGroup = document.getElementById('wave-level-group');
            if (levelGroup) {
                // Precise math: Bottom of circle is Y=176, Top is Y=24. Total Range = 152px.
                // Baseline (group origin) is at Y=100.
                // Empty: translateY(76), Full: translateY(-76)
                const translateY = 76 - (152 * progressRatio);
                levelGroup.style.transform = `translateY(${translateY}px)`;
            }
        } else if (theme === 'hourglass') {
            const topGroup = document.getElementById('sand-top-level-group');
            const bottomGroup = document.getElementById('sand-bottom-level-group');

            if (topGroup && bottomGroup) {
                // Top chamber: Drains from full (y=40) to empty (y=100)
                // Path baseline is at 100. Translate from -60 to 0.
                const topY = -60 + (60 * progressRatio);
                topGroup.style.transform = `translateY(${topY}px)`;

                // Bottom chamber: Fills from empty (y=160) to full (y=100)
                // Path baseline is at 100. Translate from 60 to 0.
                const bottomY = 60 - (60 * progressRatio);
                bottomGroup.style.transform = `translateY(${bottomY}px)`;

                // Dynamically update stream feather mask endpoint (y2) to extend further down (+26px) into the sand wave
                // Ensures stream penetrates deep into the wave with a seamless soft-feathered fade out
                const streamMaskGrad = document.getElementById('stream-mask-grad');
                if (streamMaskGrad) {
                    const sandSurfaceY = Math.max(98, 160 - (60 * progressRatio));
                    streamMaskGrad.setAttribute('y2', (sandSurfaceY + 26).toFixed(1));
                }

                const isFlowing = activeState !== 'idle' && !isPaused && ratio > 0;
                updateSandStreamState(isFlowing);
            }
        } else if (theme === 'orbit') {
            const particle = document.getElementById('orbit-particle');
            if (particle) {
                const angle = ratio * 360;
                particle.style.transform = `rotate(${angle}deg)`;
                particle.style.display = 'block';
            }
        }
    }

    // Realtime GIF Theme & Display Mode Renderer
    function applyGifTheme() {
        const opacity = (userData.gifOpacity || 78) / 100;
        const isFullMode = (userData.gifDisplayMode === 'full');
        const plantGrowthContainer = document.getElementById('plant-growth-container');
        const defaultChip = document.querySelector('.gif-chip[data-gif="none"]');
        const ytMusicChip = document.querySelector('.gif-chip[data-gif="ytmusic"]');

        const vinylDiscContainer = document.getElementById('vinyl-disc-container');
        const cardVinylContainer = document.getElementById('card-vinyl-container');
        const vinylSpeedSection = document.getElementById('vinyl-speed-section');

        // Apply Spindle Visibility
        document.body.classList.toggle('hide-vinyl-spindle', !userData.showVinylSpindle);
        const chkSpindle = document.getElementById('chk-vinyl-spindle-toggle');
        if (chkSpindle) chkSpindle.checked = !!userData.showVinylSpindle;

        const currentSpeed = parseFloat(userData.vinylSpeed) || 6;
        const currentSize = userData.vinylSize || 340;
        document.documentElement.style.setProperty('--vinyl-speed', `${currentSpeed}s`);
        document.documentElement.style.setProperty('--vinyl-size', `${currentSize}px`);

        const vinylSpeedSlider = document.getElementById('vinyl-speed-slider');
        const vinylSpeedLabel = document.getElementById('vinyl-speed-label');
        if (vinylSpeedSlider) vinylSpeedSlider.value = currentSpeed;
        if (vinylSpeedLabel) vinylSpeedLabel.textContent = `${currentSpeed}s`;

        const vinylSizeSlider = document.getElementById('vinyl-size-slider');
        const vinylSizeLabel = document.getElementById('vinyl-size-label');
        if (vinylSizeSlider) vinylSizeSlider.value = currentSize;
        if (vinylSizeLabel) vinylSizeLabel.textContent = `${currentSize}px`;

        // Reset visibility: hide all ambient containers by default
        if (gaugeGifContainer) gaugeGifContainer.style.display = 'none';
        if (cardGifContainer) cardGifContainer.style.display = 'none';
        if (vinylDiscContainer) vinylDiscContainer.style.display = 'none';
        if (cardVinylContainer) cardVinylContainer.style.display = 'none';
        if (plantGrowthContainer) plantGrowthContainer.style.display = 'none';

        const recentGifsSection = document.getElementById('recent-gifs-section');
        const plantInfoBadge = document.getElementById('plant-info-badge');

        if (defaultChip) defaultChip.classList.remove('active');
        if (ytMusicChip) ytMusicChip.classList.remove('active');
        if (btnSelectCustomGif) btnSelectCustomGif.classList.remove('active');

        if (userData.ambientMode === 'ytmusic') {
            if (ytMusicChip) ytMusicChip.classList.add('active');

            if (isFullMode) {
                if (cardVinylContainer) cardVinylContainer.style.display = 'block';
            } else {
                if (vinylDiscContainer) vinylDiscContainer.style.display = 'block';
            }

            if (gifPreviewWrapper) gifPreviewWrapper.style.display = 'none';
            if (recentGifsSection) recentGifsSection.style.display = 'none';
            if (plantInfoBadge) plantInfoBadge.style.display = 'none';
            if (gifStyleSection) gifStyleSection.style.display = 'block';
            if (vinylSpeedSection) vinylSpeedSection.style.display = 'block';
            if (gifOpacityRow) gifOpacityRow.style.display = 'none';
            if (customGifNameDisplay) customGifNameDisplay.textContent = '';

            document.querySelectorAll('.speed-chip').forEach(chip => {
                chip.classList.toggle('active', parseFloat(chip.getAttribute('data-speed')) === currentSpeed);
            });
            setTimeout(syncVinylCenterPosition, 10);

            document.querySelectorAll('.size-preset-chip').forEach(chip => {
                chip.classList.toggle('active', parseInt(chip.getAttribute('data-size')) === currentSize);
            });

            document.querySelectorAll('.gif-mode-chip').forEach(chip => {
                chip.classList.toggle('active', chip.getAttribute('data-mode') === (userData.gifDisplayMode || 'circle'));
            });

            updateYtMusicUI();

        } else if (userData.ambientMode === 'custom') {
            if (btnSelectCustomGif) btnSelectCustomGif.classList.add('active');
            if (plantInfoBadge) plantInfoBadge.style.display = 'none';
            if (recentGifsSection) recentGifsSection.style.display = (userData.recentGifs && userData.recentGifs.length > 0) ? 'block' : 'none';
            if (gifStyleSection) gifStyleSection.style.display = 'block';
            if (vinylSpeedSection) vinylSpeedSection.style.display = 'none';
            if (gifOpacityRow) gifOpacityRow.style.display = 'block';
            if (gifOpacitySlider) gifOpacitySlider.value = userData.gifOpacity || 78;
            if (opacityLabel) opacityLabel.textContent = `${userData.gifOpacity || 78}%`;

            if (userData.customGifData && (userData.customGifData.length > 50 || !userData.customGifData.startsWith('data:'))) {
                // Load data from IndexedDB or use directly if it's still base64
                (async () => {
                    let actualData = userData.customGifData;
                    if (actualData && !actualData.startsWith('data:image')) {
                        actualData = await gifDb.get(actualData);
                    }

                    if (!actualData) return;
                    window.cachedCustomGifData = actualData;
                    if (typeof updatePipPeekingVinyl === 'function') {
                        updatePipPeekingVinyl();
                    }

                    if (isFullMode) {
                        cardGifImg.src = actualData;
                        cardGifImg.style.opacity = opacity;
                        cardGifContainer.style.display = 'block';
                    } else {
                        gaugeGifImg.src = actualData;
                        gaugeGifImg.style.opacity = opacity;
                        gaugeGifContainer.style.display = 'block';
                    }

                    // Sync modal preview thumbnail & controls
                    if (gifPreviewImg) {
                        gifPreviewImg.src = actualData;
                        gifPreviewImg.style.opacity = opacity;
                    }
                    if (gifPreviewContainer) {
                        gifPreviewContainer.style.borderRadius = isFullMode ? '8px' : '50%';
                    }
                    if (gifPreviewWrapper) gifPreviewWrapper.style.display = 'block';
                    if (customGifNameDisplay) customGifNameDisplay.textContent = userData.customGifName || 'Custom GIF';

                    document.querySelectorAll('.gif-mode-chip').forEach(chip => {
                        chip.classList.toggle('active', chip.getAttribute('data-mode') === (userData.gifDisplayMode || 'circle'));
                    });
                })();
            } else {
                if (gifPreviewWrapper) gifPreviewWrapper.style.display = 'none';
                if (customGifNameDisplay) customGifNameDisplay.textContent = '';
            }
        } else {
            if (defaultChip) defaultChip.classList.add('active');
            if (plantGrowthContainer) plantGrowthContainer.style.display = 'flex';
            if (plantInfoBadge) plantInfoBadge.style.display = 'block';
            if (recentGifsSection) recentGifsSection.style.display = 'none';
            if (gifPreviewWrapper) gifPreviewWrapper.style.display = 'none';
            if (gifStyleSection) gifStyleSection.style.display = 'none';
            if (vinylSpeedSection) vinylSpeedSection.style.display = 'none';
            if (gifOpacityRow) gifOpacityRow.style.display = 'none';
            if (customGifNameDisplay) customGifNameDisplay.textContent = '';
        }

        // Always keep YouTube Music tickers & side peeking vinyl overlay in sync
        updateYtMusicUI();

        // In-place synchronization of active state on Recent GIF cards (Zero-Flicker)
        document.querySelectorAll('.recent-gif-card').forEach(c => {
            const isThisActive = (userData.ambientMode === 'custom' && c.getAttribute('data-gif-data') === userData.customGifData);
            c.classList.toggle('active', isThisActive);
        });
    }

    // Render Recent GIFs History (Max 5 items)
    async function renderRecentGifs() {
        try {
            const recentGifsSection = document.getElementById('recent-gifs-section');
            const recentGifsContainer = document.getElementById('recent-gifs-container');
            if (!recentGifsSection || !recentGifsContainer) return;

            userData.recentGifs = userData.recentGifs || [];

            // Auto-add current active GIF to recent list if not present
            if (userData.customGifData && (userData.customGifData.length > 50 || !userData.customGifData.startsWith('data:'))) {
                const exists = userData.recentGifs.some(g => g.data === userData.customGifData);
                if (!exists) {
                    const newId = userData.customGifData.startsWith('data:') ? 'gif_' + Date.now() : userData.customGifData;
                    if (userData.customGifData.startsWith('data:')) {
                        await gifDb.save(newId, userData.customGifData);
                        userData.customGifData = newId;
                    }

                    userData.recentGifs.unshift({
                        id: newId,
                        name: userData.customGifName || 'Custom GIF',
                        data: newId
                    });
                    if (userData.recentGifs.length > 5) {
                        const removed = userData.recentGifs.pop();
                        // Optional: Clean up removed GIF from IndexedDB if not active
                        if (removed.data !== userData.customGifData) {
                            await gifDb.delete(removed.data);
                        }
                    }
                    saveUserData();
                }
            }

            if (userData.recentGifs.length === 0) {
                recentGifsSection.style.display = 'none';
                recentGifsContainer.innerHTML = '';
                return;
            }

            recentGifsSection.style.display = 'block';

            // Fetch all thumbnails in parallel before touching DOM to avoid blank reload flicker
            const itemsWithThumbs = await Promise.all(userData.recentGifs.map(async (gif) => {
                try {
                    const thumbData = gif.data.startsWith('data:') ? gif.data : await gifDb.get(gif.data);
                    return { gif, thumbData };
                } catch (e) {
                    return { gif, thumbData: '' };
                }
            }));

            const fragment = document.createDocumentFragment();

            itemsWithThumbs.forEach(({ gif, thumbData }, index) => {
                const isActive = (userData.ambientMode === 'custom' && userData.customGifData === gif.data);
                const card = document.createElement('div');
                card.className = `recent-gif-card ${isActive ? 'active' : ''}`;
                card.setAttribute('data-gif-data', gif.data);
                card.title = gif.name;

                card.innerHTML = `
                    <img class="recent-gif-thumb" src="${thumbData || ''}" alt="thumb">
                    <div class="recent-gif-info">
                        <span class="recent-gif-title">${gif.name}</span>
                    </div>
                    <button class="btn-remove-recent" title="Remove from history">&times;</button>
                `;

                // Click card to switch to this GIF (Zero-Flicker in-place active switch)
                card.addEventListener('click', (e) => {
                    if (e.target.classList.contains('btn-remove-recent')) return;
                    userData.ambientMode = 'custom';
                    userData.customGifData = gif.data;
                    userData.customGifName = gif.name;
                    saveUserData();
                    applyGifTheme();
                    syncCurrentStateToFirebase(true);
                });

                // Click remove button
                card.querySelector('.btn-remove-recent')?.addEventListener('click', async (e) => {
                    e.stopPropagation();
                    const removed = userData.recentGifs.splice(index, 1)[0];
                    await gifDb.delete(removed.data);

                    if (isActive) {
                        if (userData.recentGifs.length > 0) {
                            userData.ambientMode = 'custom';
                            userData.customGifData = userData.recentGifs[0].data;
                            userData.customGifName = userData.recentGifs[0].name;
                        } else {
                            userData.ambientMode = 'plant';
                            userData.customGifData = '';
                            userData.customGifName = '';
                        }
                    }
                    saveUserData();
                    applyGifTheme();
                    renderRecentGifs();
                    syncCurrentStateToFirebase(true);
                });

                fragment.appendChild(card);
            });

            recentGifsContainer.replaceChildren(fragment);
        } catch (err) {
            console.warn('[GIF] renderRecentGifs safe caught error:', err);
        }
    }

    applyGifTheme();
    applyTimerTheme();
    renderRecentGifs();
    updateYtMusicUI();

    // Auto-sync current active GIF & stats to Android / Firebase on startup
    setTimeout(() => {
        syncCurrentStateToFirebase(true);
    }, 1200);

    // Clear Recent GIFs Button Listener
    document.getElementById('btn-clear-recent-gifs')?.addEventListener('click', async () => {
        userData.recentGifs = [];
        await gifDb.clearAll();
        // If current active is custom, reset it too
        if (userData.ambientMode === 'custom') {
            userData.ambientMode = 'plant';
            userData.customGifData = '';
            userData.customGifName = '';
        }
        saveUserData();
        applyGifTheme();
        renderRecentGifs();
        syncCurrentStateToFirebase(true);
    });

    // YT Music Vinyl Preset Button
    document.querySelector('.gif-chip[data-gif="ytmusic"]')?.addEventListener('click', () => {
        userData.ambientMode = 'ytmusic';
        saveUserData();
        applyGifTheme();
        syncCurrentStateToFirebase(true);
    });

    // File Upload / Mode Switch Handler
    btnSelectCustomGif.addEventListener('click', () => {
        const hasHistory = (userData.recentGifs && userData.recentGifs.length > 0);
        const hasCustomData = !!(userData.customGifData && (userData.customGifData.length > 50 || !userData.customGifData.startsWith('data:')));

        // If already in custom mode, clicking it allows picking a new file
        if (userData.ambientMode === 'custom') {
            gifFileInput.click();
            return;
        }

        // Switch to custom mode
        userData.ambientMode = 'custom';
        if (!hasCustomData && hasHistory) {
            userData.customGifData = userData.recentGifs[0].data;
            userData.customGifName = userData.recentGifs[0].name;
        }

        saveUserData();
        applyGifTheme();
        renderRecentGifs();
        syncCurrentStateToFirebase(true);

        // If user has never selected/uploaded any GIF, open file dialog immediately
        if (!hasCustomData && !hasHistory) {
            gifFileInput.click();
        }
    });

    // Upload New GIF Button inside Recent GIFs Section
    document.getElementById('btn-upload-new-gif')?.addEventListener('click', () => {
        gifFileInput.click();
    });

    gifFileInput.addEventListener('change', (e) => {
        const file = e.target.files[0];
        if (!file) return;

        const reader = new FileReader();
        reader.onload = async (evt) => {
            const newData = evt.target.result;
            const newName = file.name;
            const newId = 'gif_' + Date.now();

            await gifDb.save(newId, newData);

            userData.ambientMode = 'custom';
            userData.customGifData = newId;
            userData.customGifName = newName;
            userData.gifOpacity = userData.gifOpacity || 78;
            userData.gifDisplayMode = userData.gifDisplayMode || 'circle';

            userData.recentGifs = userData.recentGifs || [];
            // Remove if exists (by data/id)
            userData.recentGifs = userData.recentGifs.filter(g => g.data !== newId);

            userData.recentGifs.unshift({
                id: newId,
                name: newName,
                data: newId
            });
            if (userData.recentGifs.length > 5) {
                const removed = userData.recentGifs.pop();
                if (removed.data !== userData.customGifData) {
                    await gifDb.delete(removed.data);
                }
            }

            saveUserData();
            applyGifTheme();
            renderRecentGifs();
            syncCurrentStateToFirebase(true);
        };
        reader.readAsDataURL(file);
    });

    // Realtime Opacity Slider Listener
    gifOpacitySlider?.addEventListener('input', (e) => {
        userData.gifOpacity = parseInt(e.target.value);
        const opacity = userData.gifOpacity / 100;

        if (opacityLabel) opacityLabel.textContent = `${userData.gifOpacity}%`;
        if (gaugeGifImg) gaugeGifImg.style.opacity = opacity;
        if (cardGifImg) cardGifImg.style.opacity = opacity;
        if (gifPreviewImg) gifPreviewImg.style.opacity = opacity;

        saveUserData();
    });

    gifOpacitySlider?.addEventListener('change', () => {
        syncCurrentStateToFirebase(true);
    });

    // Display Style Mode Switcher (Circle vs Full Panel)
    document.querySelectorAll('.gif-mode-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            userData.gifDisplayMode = chip.getAttribute('data-mode');
            saveUserData();
            applyGifTheme();
        });
    });

    // Center Spindle Toggle Listener
    document.getElementById('chk-vinyl-spindle-toggle')?.addEventListener('change', (e) => {
        userData.showVinylSpindle = e.target.checked;
        saveUserData();
        document.body.classList.toggle('hide-vinyl-spindle', !userData.showVinylSpindle);
    });

    // Realtime Vinyl Rotation Speed Slider Listener
    document.getElementById('vinyl-speed-slider')?.addEventListener('input', (e) => {
        userData.vinylSpeed = parseFloat(e.target.value);
        document.documentElement.style.setProperty('--vinyl-speed', `${userData.vinylSpeed}s`);
        const label = document.getElementById('vinyl-speed-label');
        if (label) label.textContent = `${userData.vinylSpeed}s`;

        document.querySelectorAll('.speed-chip').forEach(chip => {
            chip.classList.toggle('active', parseFloat(chip.getAttribute('data-speed')) === userData.vinylSpeed);
        });

        saveUserData();
        updatePipPeekingVinyl();
    });

    // Vinyl Rotation Speed Selector Switcher
    document.querySelectorAll('.speed-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            userData.vinylSpeed = parseFloat(chip.getAttribute('data-speed')) || 6;
            saveUserData();
            applyGifTheme();
        });
    });

    // Realtime Vinyl Disc Size Slider Listener
    document.getElementById('vinyl-size-slider')?.addEventListener('input', (e) => {
        userData.vinylSize = parseInt(e.target.value);
        document.documentElement.style.setProperty('--vinyl-size', `${userData.vinylSize}px`);
        const label = document.getElementById('vinyl-size-label');
        if (label) label.textContent = `${userData.vinylSize}px`;

        document.querySelectorAll('.size-preset-chip').forEach(chip => {
            chip.classList.toggle('active', parseInt(chip.getAttribute('data-size')) === userData.vinylSize);
        });

        saveUserData();
    });

    // Vinyl Size Preset Chips Switcher
    document.querySelectorAll('.size-preset-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            userData.vinylSize = parseInt(chip.getAttribute('data-size')) || 340;
            saveUserData();
            applyGifTheme();
        });
    });

    // Accent Color Preset Chips Listener (Selects Base Color)
    document.querySelectorAll('.accent-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            const accentVal = chip.getAttribute('data-accent');
            userData.accentColor = accentVal;
            if (userData.accentMode !== 'ytmusic_dynamic' && !userData.dynamicAlbumEnabled) {
                userData.accentMode = 'preset';
            }
            document.querySelectorAll('.accent-chip').forEach(c => c.classList.remove('active'));
            chip.classList.add('active');
            saveUserData();
            updateYtMusicUI();
            if (!userData.dynamicAlbumEnabled && userData.accentMode !== 'ytmusic_dynamic') {
                applyAccentTheme(accentVal);
            }
        });
    });

    // Custom Color Picker Listener (Selects Base Color)
    document.getElementById('accent-color-picker')?.addEventListener('input', (e) => {
        const hex = e.target.value;
        userData.accentColor = hex;
        if (userData.accentMode !== 'ytmusic_dynamic' && !userData.dynamicAlbumEnabled) {
            userData.accentMode = 'custom';
        }
        document.querySelectorAll('.accent-chip').forEach(c => c.classList.remove('active'));
        saveUserData();
        updateYtMusicUI();
        if (!userData.dynamicAlbumEnabled && userData.accentMode !== 'ytmusic_dynamic') {
            applyAccentTheme(hex);
        }
    });

    // Dynamic YT Music Album Sync Toggle Switch Listener
    const chkDynamicAlbum = document.getElementById('chk-dynamic-album');
    if (chkDynamicAlbum) {
        chkDynamicAlbum.checked = !!(userData.dynamicAlbumEnabled || userData.accentMode === 'ytmusic_dynamic');
        chkDynamicAlbum.addEventListener('change', (e) => {
            userData.dynamicAlbumEnabled = e.target.checked;
            userData.accentMode = e.target.checked ? 'ytmusic_dynamic' : 'preset';
            saveUserData();
            updateYtMusicUI();
            if (!e.target.checked) {
                applyAccentTheme(userData.accentColor || '#60cdff');
            }
        });
    }

    // High-Contrast Complementary Accent Toggle Listener
    const chkDynamicContrast = document.getElementById('chk-dynamic-contrast');
    if (chkDynamicContrast) {
        chkDynamicContrast.checked = (userData.useComplementaryColor !== false);
        chkDynamicContrast.addEventListener('change', (e) => {
            userData.useComplementaryColor = e.target.checked;
            saveUserData();
            updateYtMusicUI();
        });
    }

    // Auto-Rotate Hourglass Every 5s Toggle Listener
    const chkHourglassRotate = document.getElementById('chk-hourglass-rotate');
    if (chkHourglassRotate) {
        chkHourglassRotate.checked = !!userData.hourglassAutoRotate;
        chkHourglassRotate.addEventListener('change', (e) => {
            userData.hourglassAutoRotate = e.target.checked;
            saveUserData();
            applyTimerTheme();
        });
    }

    // Hover to Peek Timer in Stealth Mode Switch Listener
    const chkStealthHoverPeek = document.getElementById('chk-stealth-hover-peek');
    if (chkStealthHoverPeek) {
        chkStealthHoverPeek.checked = userData.stealthHoverPeek !== false;
        chkStealthHoverPeek.addEventListener('change', (e) => {
            userData.stealthHoverPeek = e.target.checked;
            saveUserData();
            applyTimerTheme();
        });
    }


    window.handlePipStopClick = function() {
        const btnStop = document.getElementById('btn-stop');
        if (btnStop) btnStop.click();
    };

    window.handleExternalVinylMedia = function(action) {
        if (action === 'prev') {
            const btn = document.querySelector('.btn-yt-prev');
            if (btn) btn.click();
        } else if (action === 'playpause') {
            const btn = document.querySelector('.btn-yt-playpause');
            if (btn) btn.click();
        } else if (action === 'next') {
            const btn = document.querySelector('.btn-yt-next');
            if (btn) btn.click();
        }
    };

    // PIP Peeking Vinyl Toggle Listener
    const chkPipPeekingVinyl = document.getElementById('chk-pip-peeking-vinyl');
    if (chkPipPeekingVinyl) {
        chkPipPeekingVinyl.checked = !!userData.pipPeekingVinylEnabled;
        chkPipPeekingVinyl.addEventListener('change', (e) => {
            userData.pipPeekingVinylEnabled = e.target.checked;
            saveUserData();
            updatePipPeekingVinyl();
        });
    }

    // PIP Peeking Vinyl Side Selector Listeners
    document.querySelectorAll('.pip-side-btn').forEach(btn => {
        const side = btn.getAttribute('data-side');
        btn.classList.toggle('active', side === (userData.pipPeekingVinylSide || 'left'));
        btn.addEventListener('click', () => {
            document.querySelectorAll('.pip-side-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            userData.pipPeekingVinylSide = side;
            saveUserData();
            updatePipPeekingVinyl();
        });
    });

    // YT Music Media Control Buttons (Prev, Play/Pause, Next)
    let lastMediaNavTime = 0;
    let lastMediaPlayPauseTime = 0;

    document.querySelectorAll('.btn-yt-prev').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            const now = Date.now();
            if (now - lastMediaNavTime < 350) return;
            lastMediaNavTime = now;
            console.log(`[Media] Prev clicked (Source: ${ytTrackData.source || 'default'}, WS: ${ytmpxSocket?.readyState === WebSocket.OPEN})`);
            if (ytmpxSocket && ytmpxSocket.readyState === WebSocket.OPEN) {
                console.log('[YTMPX] Sending prev command to Tauri WebSocket');
                ytmpxSocket.send(JSON.stringify({ command: 'prev' }));
            } else {
                console.log('[Media] Controlling Browser/System via C++ system media key');
                sendToCpp({ action: 'mediaPrev' });
            }
        });
    });

    document.querySelectorAll('.btn-yt-playpause').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            const now = Date.now();
            if (now - lastMediaPlayPauseTime < 350) {
                console.log('[Media] PlayPause click debounced (<350ms)');
                return;
            }
            lastMediaPlayPauseTime = now;
            console.log(`[Media] PlayPause clicked (Source: ${ytTrackData.source || 'default'}, WS: ${ytmpxSocket?.readyState === WebSocket.OPEN})`);

            // Optimistic UI feedback so button state responds instantly without waiting for WebSocket roundtrip
            ytTrackData.isPlaying = !ytTrackData.isPlaying;
            updateYtMusicUI();

            if (ytmpxSocket && ytmpxSocket.readyState === WebSocket.OPEN) {
                console.log('[YTMPX] Sending playPause command to Tauri WebSocket');
                ytmpxSocket.send(JSON.stringify({ command: 'playPause' }));
            } else {
                console.log('[Media] Controlling Browser/System via C++ system media key');
                sendToCpp({ action: 'mediaPlayPause' });
            }
        });
    });

    document.querySelectorAll('.btn-yt-next').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            const now = Date.now();
            if (now - lastMediaNavTime < 350) return;
            lastMediaNavTime = now;
            console.log(`[Media] Next clicked (Source: ${ytTrackData.source || 'default'}, WS: ${ytmpxSocket?.readyState === WebSocket.OPEN})`);
            if (ytmpxSocket && ytmpxSocket.readyState === WebSocket.OPEN) {
                console.log('[YTMPX] Sending next command to Tauri WebSocket');
                ytmpxSocket.send(JSON.stringify({ command: 'next' }));
            } else {
                console.log('[Media] Controlling Browser/System via C++ system media key');
                sendToCpp({ action: 'mediaNext' });
            }
        });
    });

    // Vinyl Disc Click: Toggle / Bring to front or minimize active Music window/tab
    let lastVinylClickTime = 0;
    const handleVinylDiscClick = (e) => {
        if (e) e.stopPropagation();
        const now = Date.now();
        if (now - lastVinylClickTime < 250) {
            console.log('[Media] Vinyl Disc click debounced (<250ms)');
            return;
        }
        lastVinylClickTime = now;

        console.log('[Media] Vinyl Disc clicked -> Toggling Music Window (Title:', ytTrackData.title, ', Source:', ytTrackData.source, ', Exe:', ytTrackData.exeName, ')');
        
        // If connected to Tauri WebSocket or source is Tauri, communicate directly via WebSocket and NEVER call C++
        const isTauriSource = (ytmpxSocket && ytmpxSocket.readyState === WebSocket.OPEN) ||
                              ytTrackData.source === 'tauri' ||
                              (ytTrackData.exeName && ytTrackData.exeName.toLowerCase().includes('ytm-tauri'));
        if (isTauriSource && ytmpxSocket && ytmpxSocket.readyState === WebSocket.OPEN) {
            ytmpxSocket.send(JSON.stringify({ command: 'toggleMusicWindow' }));
            return;
        }

        // Notify C++ to handle window activation/minimize for Browser tabs
        sendToCpp({
            action: 'toggleMusicWindow',
            title: ytTrackData.title || '',
            exeName: ytTrackData.exeName || '',
            source: ytTrackData.source || ''
        });
    };

    window.handleExternalVinylClick = handleVinylDiscClick;

    // Vinyl Disc Scroll: Adjust YouTube Music Volume
    let lastVolumeScrollTime = 0;
    const handleVinylDiscWheel = (e) => {
        e.preventDefault();
        e.stopPropagation();
        lastVolumeScrollTime = Date.now();
        const direction = e.deltaY < 0 ? 'volumeUp' : 'volumeDown';
        console.log(`[Media] Vinyl Wheel: ${direction} (Source: ${ytTrackData.source || 'default'})`);
        if (ytmpxSocket && ytmpxSocket.readyState === WebSocket.OPEN) {
            ytmpxSocket.send(JSON.stringify({ command: direction }));
        } else {
            sendToCpp({ action: direction === 'volumeUp' ? 'mediaVolumeUp' : 'mediaVolumeDown' });
        }
    };

    window.handleExternalVinylWheel = (direction) => {
        console.log(`[Media] External Vinyl Wheel: ${direction} (Source: ${ytTrackData.source || 'default'})`);
        lastVolumeScrollTime = Date.now();
        if (ytmpxSocket && ytmpxSocket.readyState === WebSocket.OPEN) {
            ytmpxSocket.send(JSON.stringify({ command: direction }));
        } else {
            sendToCpp({ action: direction === 'volumeUp' ? 'mediaVolumeUp' : 'mediaVolumeDown' });
        }
    };

    document.querySelectorAll('.card-vinyl-disc, .vinyl-disc, #card-vinyl-container, #vinyl-disc-container, .pip-peeking-vinyl-disc, .card-vinyl-backdrop, .ticker-content').forEach(el => {
        el.addEventListener('click', handleVinylDiscClick);
        el.addEventListener('wheel', handleVinylDiscWheel, { passive: false });
        el.setAttribute('title', 'Click to toggle Music player • Scroll to adjust Volume');
    });

    document.querySelectorAll('.ytmusic-ticker').forEach(ticker => {
        ticker.addEventListener('click', (e) => {
            if (!e.target.closest('.media-control-btn')) {
                handleVinylDiscClick(e);
            }
        });
        ticker.addEventListener('wheel', (e) => {
            if (!e.target.closest('.media-control-btn')) {
                handleVinylDiscWheel(e);
            }
        }, { passive: false });
        ticker.setAttribute('title', 'Click to toggle Music player • Scroll to adjust Volume');
    });

    // None GIF Preset Button
    document.querySelector('.gif-chip[data-gif="none"]')?.addEventListener('click', () => {
        userData.ambientMode = 'plant';
        userData.customGifData = '';
        userData.customGifName = '';
        saveUserData();
        applyGifTheme();
    });

    // Generate Gauge Radial Ticks
    function renderGaugeTicks() {
        if (!gaugeTicks) return;
        gaugeTicks.innerHTML = '';
        const totalTicks = 28;
        const cx = 100, cy = 100, r = 88;

        for (let i = 0; i < totalTicks; i++) {
            const angle = (i * 360 / totalTicks) * (Math.PI / 180);
            const x1 = cx + (r - 6) * Math.cos(angle);
            const y1 = cy + (r - 6) * Math.sin(angle);
            const x2 = cx + r * Math.cos(angle);
            const y2 = cy + r * Math.sin(angle);

            const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
            line.setAttribute('x1', x1);
            line.setAttribute('y1', y1);
            line.setAttribute('x2', x2);
            line.setAttribute('y2', y2);
            line.setAttribute('stroke', 'rgba(255, 255, 255, 0.15)');
            line.setAttribute('stroke-width', '2');
            line.setAttribute('stroke-linecap', 'round');
            gaugeTicks.appendChild(line);
        }
    }
    renderGaugeTicks();

    // Time Picker Controls
    function updateSetupPrediction() {
        if (!sessionPredictionText) return;

        const periods = Math.ceil(selectedMins / selectedPeriodMins);
        const breaks = (periods > 1 && !chkSkipBreaks.checked) ? (periods - 1) : 0;
        const totalBreakMins = breaks * selectedBreakMins;
        const totalDurationMins = selectedMins + totalBreakMins;

        const now = new Date();
        const finishDate = new Date(now.getTime() + totalDurationMins * 60 * 1000);

        const hours = String(finishDate.getHours()).padStart(2, '0');
        const minutes = String(finishDate.getMinutes()).padStart(2, '0');
        const timeStr = `${hours}:${minutes}`;

        const isNextDay = finishDate.getDate() !== now.getDate() || finishDate.getMonth() !== now.getMonth();
        const finishHour = finishDate.getHours();
        // Night / bedtime window (between 23:00 and 06:00)
        const isLateNight = (finishHour >= 23 || finishHour < 6);

        const durH = Math.floor(totalDurationMins / 60);
        const durM = totalDurationMins % 60;
        let durStr = '';
        if (durH > 0 && durM > 0) durStr = `${durH}h ${durM}m`;
        else if (durH > 0) durStr = `${durH}h`;
        else durStr = `${durM}m`;

        const daySuffix = isNextDay ? ' (+1d)' : '';
        const nextPredictionHtml = `Ends at <strong>${timeStr}</strong>${daySuffix} • ${durStr} total`;
        if (sessionPredictionText._lastHtml !== nextPredictionHtml) {
            sessionPredictionText._lastHtml = nextPredictionHtml;
            sessionPredictionText.innerHTML = nextPredictionHtml;
        }

        if (sessionPredictionBadge) {
            sessionPredictionBadge.classList.toggle('late-night', isLateNight);
            if (predictionIconClock && predictionIconMoon) {
                predictionIconClock.style.display = isLateNight ? 'none' : 'block';
                predictionIconMoon.style.display = isLateNight ? 'block' : 'none';
            }
        }
    }

    function updatePickerDisplay() {
        if (pickerMinsInput && document.activeElement !== pickerMinsInput) {
            pickerMinsInput.value = selectedMins;
        }
        if (pickerPeriodInput && document.activeElement !== pickerPeriodInput) {
            pickerPeriodInput.value = selectedPeriodMins;
        }
        if (pickerBreakInput && document.activeElement !== pickerBreakInput) {
            pickerBreakInput.value = selectedBreakMins;
        }

        const periods = Math.ceil(selectedMins / selectedPeriodMins);
        const breaks = (periods > 1 && !chkSkipBreaks.checked) ? (periods - 1) : 0;

        if (breaks > 0) {
            const fullPeriods = Math.floor(selectedMins / selectedPeriodMins);
            const remainderMins = selectedMins % selectedPeriodMins;
            if (remainderMins > 0 && fullPeriods > 0) {
                breaksCountText.textContent = `You'll have ${breaks} break${breaks > 1 ? 's' : ''} (${selectedBreakMins} mins each) across ${fullPeriods} full period${fullPeriods > 1 ? 's' : ''} (${selectedPeriodMins} mins) + 1 final period (${remainderMins} mins)`;
            } else {
                breaksCountText.textContent = `You'll have ${breaks} break${breaks > 1 ? 's' : ''} (${selectedBreakMins} mins each) across ${periods} focus periods (${selectedPeriodMins} mins each)`;
            }
        } else {
            breaksCountText.textContent = `Continuous ${selectedMins} min focus session (${selectedPeriodMins} min period)`;
        }

        updateSetupPrediction();

        userData.selectedMins = selectedMins;
        userData.selectedPeriodMins = selectedPeriodMins;
        userData.selectedBreakMins = selectedBreakMins;
        if (chkSkipBreaks) {
            userData.skipBreaks = chkSkipBreaks.checked;
        }
        saveUserData();
    }

    // Direct Typing & Keyboard Interaction Event Handlers
    [pickerMinsInput, pickerPeriodInput, pickerBreakInput].forEach(input => {
        if (!input) return;
        input.addEventListener('focus', () => input.select());
    });

    pickerMinsInput?.addEventListener('input', (e) => {
        let val = parseInt(e.target.value);
        if (!isNaN(val) && val > 0) {
            selectedMins = Math.min(val, 480);
            updatePickerDisplay();
        }
    });

    pickerMinsInput?.addEventListener('blur', () => {
        let val = parseInt(pickerMinsInput.value);
        if (isNaN(val) || val < 1) selectedMins = 1;
        else selectedMins = Math.min(val, 480);
        pickerMinsInput.value = selectedMins;
        updatePickerDisplay();
    });

    pickerMinsInput?.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') pickerMinsInput.blur();
    });

    pickerPeriodInput?.addEventListener('input', (e) => {
        let val = parseInt(e.target.value);
        if (!isNaN(val) && val > 0) {
            selectedPeriodMins = Math.min(val, 120);
            updatePickerDisplay();
        }
    });

    pickerPeriodInput?.addEventListener('blur', () => {
        let val = parseInt(pickerPeriodInput.value);
        if (isNaN(val) || val < 1) selectedPeriodMins = 1;
        else selectedPeriodMins = Math.min(val, 120);
        pickerPeriodInput.value = selectedPeriodMins;
        updatePickerDisplay();
    });

    pickerPeriodInput?.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') pickerPeriodInput.blur();
    });

    pickerBreakInput?.addEventListener('input', (e) => {
        let val = parseInt(e.target.value);
        if (!isNaN(val) && val > 0) {
            selectedBreakMins = Math.min(val, 60);
            updatePickerDisplay();
        }
    });

    pickerBreakInput?.addEventListener('blur', () => {
        let val = parseInt(pickerBreakInput.value);
        if (isNaN(val) || val < 1) selectedBreakMins = 1;
        else selectedBreakMins = Math.min(val, 60);
        pickerBreakInput.value = selectedBreakMins;
        updatePickerDisplay();
    });

    pickerBreakInput?.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') pickerBreakInput.blur();
    });

    if (chkSkipBreaks && userData.skipBreaks !== undefined) {
        chkSkipBreaks.checked = !!userData.skipBreaks;
    }
    chkSkipBreaks?.addEventListener('change', () => {
        updatePickerDisplay();
    });

    btnPickerUp.addEventListener('click', () => {
        if (selectedMins < 480) {
            const step = Math.max(1, selectedPeriodMins || 25);
            if (selectedMins < step) {
                if (selectedMins < 5) selectedMins = Math.min(step, selectedMins + 1);
                else selectedMins = Math.min(step, selectedMins + 5);
            } else {
                let next = Math.floor(selectedMins / step) * step + step;
                if (next <= selectedMins) next += step;
                selectedMins = Math.min(480, next);
            }
            updatePickerDisplay();
        }
    });

    btnPickerDown.addEventListener('click', () => {
        if (selectedMins > 1) {
            const step = Math.max(1, selectedPeriodMins || 25);
            if (selectedMins <= step) {
                if (selectedMins <= 5) selectedMins = Math.max(1, selectedMins - 1);
                else selectedMins = Math.max(1, selectedMins - 5);
            } else {
                let prev = Math.ceil(selectedMins / step) * step - step;
                if (prev >= selectedMins) prev -= step;
                if (prev < step) prev = step;
                selectedMins = Math.max(1, prev);
            }
            updatePickerDisplay();
        }
    });

    btnPeriodUp?.addEventListener('click', () => {
        if (selectedPeriodMins < 120) {
            if (selectedPeriodMins < 5) selectedPeriodMins += 1;
            else selectedPeriodMins += 5;
            updatePickerDisplay();
        }
    });

    btnPeriodDown?.addEventListener('click', () => {
        if (selectedPeriodMins > 1) {
            if (selectedPeriodMins <= 5) selectedPeriodMins -= 1;
            else selectedPeriodMins -= 5;
            updatePickerDisplay();
        }
    });

    btnBreakUp.addEventListener('click', () => {
        if (selectedBreakMins < 60) {
            selectedBreakMins += 5;
            updatePickerDisplay();
        }
    });

    btnBreakDown.addEventListener('click', () => {
        if (selectedBreakMins > 1) {
            if (selectedBreakMins <= 5) selectedBreakMins -= 1;
            else selectedBreakMins -= 5;
            updatePickerDisplay();
        }
    });

    // Protection Card Segmented Tab Switching
    document.querySelectorAll('.guard-tab-btn').forEach(btn => {
        btn.addEventListener('mousedown', (e) => {
            const targetTab = btn.getAttribute('data-tab');
            console.log('Tab switch triggered:', targetTab);

            document.querySelectorAll('.guard-tab-btn').forEach(b => {
                b.classList.remove('active');
                b.style.background = 'transparent';
                b.style.color = 'var(--text-secondary)';
                b.style.fontWeight = '600';
            });
            btn.classList.add('active');
            btn.style.background = 'var(--accent-blue)';
            btn.style.color = '#000';
            btn.style.fontWeight = '700';

            document.querySelectorAll('.tab-content').forEach(content => {
                content.style.display = 'none';
                content.classList.remove('active');
            });

            const activeContent = document.getElementById(`tab-content-${targetTab}`);
            if (activeContent) {
                activeContent.style.display = 'flex';
                activeContent.classList.add('active');
            }

            // Reset Prayer Display to Next Prayer whenever switching tabs
            if (window.allPrayerTimes && window.allPrayerTimes.length > 0 && typeof window.renderPrayerInfo === 'function') {
                window.displayedPrayerIndex = window.allPrayerTimes.findIndex(p => p.name === window.actualNextPrayerName);
                window.renderPrayerInfo();
            }
        });
    });

    // Restricted Websites UI Manager
    const restrictedSiteInput = document.getElementById('restricted-site-input');
    const btnAddRestrictedSite = document.getElementById('btn-add-restricted-site');
    const restrictedSitesContainer = document.getElementById('restricted-sites-container');

    function cleanDomain(input) {
        if (!input) return '';
        let d = input.trim().toLowerCase();
        d = d.replace(/^https?:\/\//, '').replace(/^www\./, '').split('/')[0].trim();
        return d;
    }

    // Sync restricted sites list to C++ engine and extension
    function syncRestrictedSitesToExtension(sites) {
        sendToCpp({ action: 'setRestrictedSites', restrictedSites: sites || [] });

        const badge = document.getElementById('ext-status-badge');
        if (badge) {
            badge.textContent = 'Synced \u2713';
            badge.style.background = 'rgba(56, 189, 248, 0.25)';
            badge.style.color = '#38bdf8';
            badge.style.borderColor = 'rgba(56, 189, 248, 0.5)';
            setTimeout(() => {
                badge.textContent = 'Extension Sync';
                badge.style.background = 'rgba(16, 185, 129, 0.15)';
                badge.style.color = '#10b981';
                badge.style.borderColor = 'rgba(16, 185, 129, 0.3)';
            }, 1200);
        }
    }

    function renderRestrictedSites() {
        if (!restrictedSitesContainer) return;
        restrictedSitesContainer.innerHTML = '';

        userData.restrictedSites = userData.restrictedSites || ['facebook.com', 'youtube.com', 'instagram.com', 'tiktok.com', 'twitter.com', 'x.com', 'reddit.com'];

        userData.restrictedSites.forEach((domain, idx) => {
            const chip = document.createElement('div');
            chip.className = 'restricted-site-chip';
            chip.style.cssText = 'background: rgba(255, 255, 255, 0.08); border: 1px solid rgba(255, 255, 255, 0.15); color: #60cdff; font-size: 11px; padding: 4px 10px; border-radius: 14px; display: inline-flex; align-items: center; gap: 6px; font-family: monospace;';

            chip.innerHTML = `
                <span>${domain}</span>
                <span class="btn-remove-site" style="cursor: pointer; opacity: 0.6; font-size: 14px; line-height: 1; margin-left: 2px;" title="Remove domain">&times;</span>
            `;

            chip.querySelector('.btn-remove-site').addEventListener('click', (e) => {
                e.stopPropagation();
                userData.restrictedSites.splice(idx, 1);
                saveUserData();
                renderRestrictedSites();
                syncRestrictedSitesToExtension(userData.restrictedSites);
                sendToCpp({ action: 'setRestrictedSites', restrictedSites: userData.restrictedSites });
            });

            restrictedSitesContainer.appendChild(chip);
        });
    }

    function handleAddRestrictedSite() {
        if (!restrictedSiteInput) return;
        const domain = cleanDomain(restrictedSiteInput.value);
        if (!domain) return;

        userData.restrictedSites = userData.restrictedSites || [];
        if (!userData.restrictedSites.includes(domain)) {
            userData.restrictedSites.push(domain);
            saveUserData();
            renderRestrictedSites();
            syncRestrictedSitesToExtension(userData.restrictedSites);
            sendToCpp({ action: 'setRestrictedSites', restrictedSites: userData.restrictedSites });
        }
        restrictedSiteInput.value = '';
    }

    if (btnAddRestrictedSite) btnAddRestrictedSite.addEventListener('click', handleAddRestrictedSite);
    if (restrictedSiteInput) {
        restrictedSiteInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') handleAddRestrictedSite();
        });
    }

    renderRestrictedSites();
    syncRestrictedSitesToExtension(userData.restrictedSites || []);
    sendToCpp({ action: 'setBlacklist', blacklist: blockedApps || [] });
    sendToCpp({ action: 'setRestrictedSites', restrictedSites: userData.restrictedSites || [] });
    sendToCpp({ action: 'setAutoCloseBlocked', enabled: userData.autoCloseBlockedApps === true });

    // Session Mode Switcher (Productive vs Health Focus)
    function syncSessionModeUI() {
        const isHealth = (userData.sessionMode === 'health');
        const btnProd = document.getElementById('btn-mode-productive');
        const btnHealth = document.getElementById('btn-mode-health');
        const setupTitle = document.getElementById('setup-view-title');
        const setupSubtitle = document.getElementById('setup-view-subtitle');
        const pickerUnit = document.getElementById('picker-mins-unit');
        const labelPeriod = document.getElementById('label-period-length');
        const labelBreak = document.getElementById('label-break-duration');
        const healthBanner = document.getElementById('health-mode-banner');
        const labelBtnStart = document.getElementById('label-btn-start');

        if (btnProd) btnProd.classList.toggle('active', !isHealth);
        if (btnHealth) btnHealth.classList.toggle('active', isHealth);

        if (isHealth) {
            if (setupTitle) {
                setupTitle.innerHTML = `
                    <span style="display:inline-flex; align-items:center; gap:8px;">
                        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="#34d399" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="12" cy="5" r="2"></circle>
                            <path d="M12 7v8"></path>
                            <path d="M9 11l3 2 3-2"></path>
                            <path d="M10 22l2-7 2 7"></path>
                        </svg>
                        <span>Posture Guard</span>
                    </span>`;
            }
            if (setupSubtitle) setupSubtitle.textContent = 'Unrestricted PC use with periodic stand & stretch reminders. Apps are never blocked.';
            if (pickerUnit) pickerUnit.textContent = 'mins session';
            if (labelPeriod) labelPeriod.textContent = 'Session length';
            if (labelBreak) labelBreak.textContent = 'Stand break duration';
            if (healthBanner) healthBanner.style.display = 'flex';
            if (labelBtnStart) labelBtnStart.textContent = 'Start posture timer';
        } else {
            if (setupTitle) setupTitle.textContent = 'Get ready to focus';
            if (setupSubtitle) setupSubtitle.textContent = "We'll block unallowed applications during each session. For longer sessions, we'll enforce short breaks so you can recharge.";
            if (pickerUnit) pickerUnit.textContent = 'mins focus';
            if (labelPeriod) labelPeriod.textContent = 'Focus period length';
            if (labelBreak) labelBreak.textContent = 'Break duration';
            if (healthBanner) healthBanner.style.display = 'none';
            if (labelBtnStart) labelBtnStart.textContent = 'Start focus session';
        }
    }

    const btnModeProd = document.getElementById('btn-mode-productive');
    const btnModeHealth = document.getElementById('btn-mode-health');
    if (btnModeProd) {
        btnModeProd.addEventListener('click', () => {
            userData.sessionMode = 'productive';
            saveUserData();
            syncSessionModeUI();
        });
    }
    if (btnModeHealth) {
        btnModeHealth.addEventListener('click', () => {
            userData.sessionMode = 'health';
            saveUserData();
            syncSessionModeUI();
        });
    }
    syncSessionModeUI();

    // Start Focus Session
    btnStartFocus.addEventListener('click', () => {
        const skipBreaks = chkSkipBreaks.checked;
        const isHealth = (userData.sessionMode === 'health');

        if (isHealth) {
            document.body.classList.add('session-health-mode');
        } else {
            document.body.classList.remove('session-health-mode');
        }

        sendToCpp({
            action: 'startSession',
            focusMinutes: selectedMins,
            focusChunkMinutes: selectedPeriodMins,
            breakMinutes: selectedBreakMins,
            skipBreaks: skipBreaks,
            isHealthMode: isHealth,
            blockAllMonitors: userData.blockAllMonitors === true,
            blacklist: isHealth ? [] : blockedApps,
            restrictedSites: isHealth ? [] : (userData.restrictedSites || [])
        });

        if (userData.autoPipOnStart) {
            isInitializingPip = true;
            setTimeout(() => { isInitializingPip = false; }, 1500);

            let pipW = userData.pipWidth || 195;
            let pipH = userData.pipHeight || 280;
            if (pipW > 440 || pipW < 150) pipW = 195;
            if (pipH > 550 || pipH < 210) pipH = 280;

            sendToCpp({
                action: 'togglePip',
                enabled: true,
                width: pipW,
                height: pipH,
                hideTaskbar: !!userData.hideTaskbarInPip
            });
        }

        setupView.classList.remove('active');
        timerView.classList.add('active');
    });

    // Pause / Stop Session
    btnPause.addEventListener('click', () => {
        sendToCpp({ action: 'pauseSession' });
    });

    btnStop.addEventListener('click', () => {
        sendToCpp({ action: 'stopSession' });
        document.body.classList.remove('session-health-mode');
        timerView.classList.remove('active');
        setupView.classList.add('active');
    });

    // Modals & Settings Options
    btnEditGoal.addEventListener('click', () => {
        goalModal.classList.add('active');
        updateHealthGoalChips();
    });
    btnCloseGoalModal.addEventListener('click', () => {
        saveUserData();
        goalModal.classList.remove('active');
    });

    document.querySelectorAll('.goal-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            const hours = parseInt(chip.getAttribute('data-hours')) || 1;
            userData.dailyGoalHours = hours;
            saveUserData();
            updateStatsUI();
            goalModal.classList.remove('active');
            goalModal.classList.add('active');
        });
    });

    function updateHealthGoalChips() {
        document.querySelectorAll('.health-goal-chip').forEach(chip => {
            const breaks = parseInt(chip.getAttribute('data-breaks')) || 4;
            chip.classList.toggle('active', (userData.healthDailyGoalBreaks || 4) === breaks);
        });
    }

    document.querySelectorAll('.health-goal-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            const breaks = parseInt(chip.getAttribute('data-breaks')) || 4;
            userData.healthDailyGoalBreaks = breaks;
            saveUserData();
            updateHealthGoalChips();
            updateStatsUI();
        });
    });

    document.querySelectorAll('.btn-options-trigger, #btn-options, #btn-options-progress').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e?.stopPropagation();
            document.querySelectorAll('.modal-overlay.active').forEach(m => m.classList.remove('active'));
            chkNotificationsToggle.checked = userData.notificationsEnabled;

            const focusSoundSelect = document.getElementById('select-focus-sound');
            if (focusSoundSelect) {
                focusSoundSelect.value = userData.focusNotificationSound || 'default';
                const focusCustomRow = document.getElementById('focus-custom-sound-row');
                if (focusCustomRow) focusCustomRow.style.display = (focusSoundSelect.value === 'custom') ? 'flex' : 'none';
            }
            const focusFileName = document.getElementById('focus-sound-file-name');
            if (focusFileName) focusFileName.textContent = userData.focusSoundFileName || 'No file chosen';

            const breakSoundSelect = document.getElementById('select-break-sound');
            if (breakSoundSelect) {
                breakSoundSelect.value = userData.breakNotificationSound || 'chime';
                const breakCustomRow = document.getElementById('break-custom-sound-row');
                if (breakCustomRow) breakCustomRow.style.display = (breakSoundSelect.value === 'custom') ? 'flex' : 'none';
            }
            const breakFileName = document.getElementById('break-sound-file-name');
            if (breakFileName) breakFileName.textContent = userData.breakSoundFileName || 'No file chosen';

            const warningSoundSelect = document.getElementById('select-warning-sound');
            if (warningSoundSelect) {
                warningSoundSelect.value = userData.warningNotificationSound || 'reminder';
                const warningCustomRow = document.getElementById('warning-custom-sound-row');
                if (warningCustomRow) warningCustomRow.style.display = (warningSoundSelect.value === 'custom') ? 'flex' : 'none';
            }
            const warningFileName = document.getElementById('warning-sound-file-name');
            if (warningFileName) warningFileName.textContent = userData.warningSoundFileName || 'No file chosen';

            const chkCountdownSound = document.getElementById('chk-countdown-sound');
            if (chkCountdownSound) chkCountdownSound.checked = userData.countdownSoundEnabled !== false;

            const selectCountdownDuration = document.getElementById('select-countdown-duration');
            if (selectCountdownDuration) selectCountdownDuration.value = String(userData.countdownDurationSec || 3);

            const selectCountdownStyle = document.getElementById('select-countdown-sound-style');
            if (selectCountdownStyle) selectCountdownStyle.value = userData.countdownSoundStyle || 'zen';

            const chkAutoPip = document.getElementById('chk-auto-pip');
            if (chkAutoPip) chkAutoPip.checked = !!userData.autoPipOnStart;

            const chkAutoPipBlur = document.getElementById('chk-auto-pip-blur');
            if (chkAutoPipBlur) chkAutoPipBlur.checked = !!userData.autoPipOnBlur;

            const chkHideTaskbarPip = document.getElementById('chk-hide-taskbar-pip');
            if (chkHideTaskbarPip) chkHideTaskbarPip.checked = !!userData.hideTaskbarInPip;

            const chkNeutralOverstay = document.getElementById('chk-neutral-overstay-toggle');
            if (chkNeutralOverstay) chkNeutralOverstay.checked = userData.neutralOverstayEnabled !== false;

            const selectNeutralInterval = document.getElementById('select-neutral-overstay-interval');
            if (selectNeutralInterval) selectNeutralInterval.value = String(userData.neutralOverstayIntervalMins || 30);

            const chkNeutralSoftBlock = document.getElementById('chk-neutral-softblock-toggle');
            if (chkNeutralSoftBlock) chkNeutralSoftBlock.checked = userData.neutralSoftBlockEnabled !== false;

            const chkStealthHoverPeek = document.getElementById('chk-stealth-hover-peek');
            if (chkStealthHoverPeek) chkStealthHoverPeek.checked = userData.stealthHoverPeek !== false;

            applyGifTheme();
            applyTimerTheme();
            updateYtMusicUI();

            // Ensure Appearance tab is default active if none active
            if (!document.querySelector('.settings-tab-btn.active')) {
                document.querySelector('.settings-tab-btn[data-tab="appearance"]')?.classList.add('active');
                document.getElementById('settings-pane-appearance')?.classList.add('active');
            }

            optionsModal.classList.add('active');
        });
    });

    // Settings Categorized Tab Switching
    document.querySelectorAll('.settings-tab-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const targetTab = btn.getAttribute('data-tab');
            document.querySelectorAll('.settings-tab-btn').forEach(b => b.classList.remove('active'));
            document.querySelectorAll('.settings-tab-pane').forEach(p => p.classList.remove('active'));
            btn.classList.add('active');
            const pane = document.getElementById(`settings-pane-${targetTab}`);
            if (pane) pane.classList.add('active');
            if (targetTab === 'mobile') {
                updateMobileSyncQrCode();
            }
        });
    });

    const selectAppLang = document.getElementById('select-app-language');
    if (selectAppLang) {
        selectAppLang.value = (userData.appLanguage === 'id') ? 'en' : (userData.appLanguage || 'en');
        selectAppLang.addEventListener('change', (e) => {
            if (e.target.value === 'id') return;
            userData.appLanguage = e.target.value;
            saveUserData();
            location.reload();
        });
    }

    // --- Mobile Sync QR Code Generator & Connection Handler ---
    let cachedLocalIp = '';

    function detectLocalIp(callback) {
        if (cachedLocalIp) {
            callback(cachedLocalIp);
            return;
        }
        try {
            const pc = new RTCPeerConnection({ iceServers: [] });
            pc.createDataChannel('');
            pc.createOffer().then(offer => pc.setLocalDescription(offer)).catch(() => {});
            let found = false;
            pc.onicecandidate = (ice) => {
                if (!ice || !ice.candidate || !ice.candidate.candidate || found) return;
                const ipMatch = /([0-9]{1,3}(\.[0-9]{1,3}){3})/.exec(ice.candidate.candidate);
                if (ipMatch && ipMatch[1] !== '127.0.0.1') {
                    found = true;
                    cachedLocalIp = ipMatch[1];
                    callback(ipMatch[1]);
                    pc.onicecandidate = null;
                }
            };
            setTimeout(() => {
                if (!found) {
                    cachedLocalIp = cachedLocalIp || '192.168.1.2';
                    callback(cachedLocalIp);
                }
            }, 1200);
        } catch (e) {
            cachedLocalIp = cachedLocalIp || '192.168.1.2';
            callback(cachedLocalIp);
        }
    }

    function updateMobileSyncQrCode() {
        const qrImg = document.getElementById('qr-code-img');
        const ipDisplay = document.getElementById('local-ip-display');
        if (!qrImg || !ipDisplay) return;

        detectLocalIp((ip) => {
            const port = 8766;
            const qrData = `http://${ip}:${port}`;
            qrImg.src = `https://api.qrserver.com/v1/create-qr-code/?size=200x200&margin=10&data=${encodeURIComponent(qrData)}`;
            ipDisplay.textContent = `PC IP: ${ip} (Port ${port})`;
        });
    }

    // Connect Phone button on Top Header Bar
    document.getElementById('btn-connect-phone')?.addEventListener('click', () => {
        document.querySelectorAll('.settings-tab-btn').forEach(b => b.classList.remove('active'));
        document.querySelectorAll('.settings-tab-pane').forEach(p => p.classList.remove('active'));
        document.querySelector('.settings-tab-btn[data-tab="mobile"]')?.classList.add('active');
        document.getElementById('settings-pane-mobile')?.classList.add('active');
        updateMobileSyncQrCode();
        optionsModal.classList.add('active');
    });

    document.getElementById('chk-auto-pip')?.addEventListener('change', (e) => {
        userData.autoPipOnStart = e.target.checked;
        saveUserData();
    });

    function updateAutoPipToggleUI() {
        const btn = document.getElementById('btn-toggle-auto-pip');
        const innerRect = document.getElementById('auto-pip-inner-rect');
        const chk = document.getElementById('chk-auto-pip-blur');
        const isEnabled = !!userData.autoPipOnBlur;

        if (chk) chk.checked = isEnabled;

        if (btn) {
            if (isEnabled) {
                btn.classList.add('active');
                btn.title = 'Auto-PiP on click outside: ON (Click to turn OFF)';
                if (innerRect) {
                    innerRect.setAttribute('fill', 'currentColor');
                    innerRect.setAttribute('fill-opacity', '0.45');
                }
            } else {
                btn.classList.remove('active');
                btn.title = 'Auto-PiP on click outside: OFF (Click to turn ON)';
                if (innerRect) {
                    innerRect.setAttribute('fill', 'none');
                    innerRect.removeAttribute('fill-opacity');
                }
            }
        }
    }

    document.getElementById('chk-auto-pip-blur')?.addEventListener('change', (e) => {
        userData.autoPipOnBlur = e.target.checked;
        saveUserData();
        updateAutoPipToggleUI();
    });

    document.getElementById('btn-toggle-auto-pip')?.addEventListener('click', () => {
        userData.autoPipOnBlur = !userData.autoPipOnBlur;
        saveUserData();
        updateAutoPipToggleUI();
    });

    document.getElementById('chk-hide-taskbar-pip')?.addEventListener('change', (e) => {
        userData.hideTaskbarInPip = e.target.checked;
        saveUserData();
    });

    // --- Work, Break & Warning Sound Option Handlers ---
    const focusSoundSelect = document.getElementById('select-focus-sound');
    const breakSoundSelect = document.getElementById('select-break-sound');
    const warningSoundSelect = document.getElementById('select-warning-sound');
    const focusCustomRow = document.getElementById('focus-custom-sound-row');
    const breakCustomRow = document.getElementById('break-custom-sound-row');
    const warningCustomRow = document.getElementById('warning-custom-sound-row');
    const focusFileInput = document.getElementById('focus-sound-file-input');
    const breakFileInput = document.getElementById('break-sound-file-input');
    const warningFileInput = document.getElementById('warning-sound-file-input');
    const focusFileName = document.getElementById('focus-sound-file-name');
    const breakFileName = document.getElementById('break-sound-file-name');
    const warningFileName = document.getElementById('warning-sound-file-name');

    focusSoundSelect?.addEventListener('change', (e) => {
        userData.focusNotificationSound = e.target.value;
        if (focusCustomRow) focusCustomRow.style.display = (e.target.value === 'custom') ? 'flex' : 'none';
        if (e.target.value === 'custom' && !window._customFocusAudio) {
            focusFileInput?.click();
        }
        saveUserData();
    });

    breakSoundSelect?.addEventListener('change', (e) => {
        userData.breakNotificationSound = e.target.value;
        if (breakCustomRow) breakCustomRow.style.display = (e.target.value === 'custom') ? 'flex' : 'none';
        if (e.target.value === 'custom' && !window._customBreakAudio) {
            breakFileInput?.click();
        }
        saveUserData();
    });

    warningSoundSelect?.addEventListener('change', (e) => {
        userData.warningNotificationSound = e.target.value;
        if (warningCustomRow) warningCustomRow.style.display = (e.target.value === 'custom') ? 'flex' : 'none';
        if (e.target.value === 'custom' && !window._customWarningAudio) {
            warningFileInput?.click();
        }
        saveUserData();
    });

    document.getElementById('btn-browse-focus-sound')?.addEventListener('click', () => focusFileInput?.click());
    document.getElementById('btn-browse-break-sound')?.addEventListener('click', () => breakFileInput?.click());
    document.getElementById('btn-browse-warning-sound')?.addEventListener('click', () => warningFileInput?.click());

    const chkCountdownSound = document.getElementById('chk-countdown-sound');
    chkCountdownSound?.addEventListener('change', (e) => {
        userData.countdownSoundEnabled = e.target.checked;
        saveUserData();
    });

    const selectCountdownDuration = document.getElementById('select-countdown-duration');
    selectCountdownDuration?.addEventListener('change', (e) => {
        userData.countdownDurationSec = parseInt(e.target.value) || 3;
        saveUserData();
        playCountdownPip(userData.countdownDurationSec);
    });

    const selectCountdownStyle = document.getElementById('select-countdown-sound-style');
    selectCountdownStyle?.addEventListener('change', (e) => {
        userData.countdownSoundStyle = e.target.value;
        saveUserData();
        playCountdownPip(1);
    });

    function handleCustomSoundFileSelect(inputElement, category) {
        const file = inputElement?.files?.[0];
        if (!file) return;

        const reader = new FileReader();
        reader.onload = async (e) => {
            const dataUrl = e.target.result;
            const dbKey = (category === 'break') ? 'break_sound' : (category === 'warning' ? 'warning_sound' : 'focus_sound');
            try {
                await gifDb.save(dbKey, dataUrl);
                if (category === 'break') {
                    window._customBreakAudio = new Audio(dataUrl);
                    userData.breakSoundFileName = file.name;
                    if (breakFileName) breakFileName.textContent = file.name;
                } else if (category === 'warning') {
                    window._customWarningAudio = new Audio(dataUrl);
                    userData.warningSoundFileName = file.name;
                    if (warningFileName) warningFileName.textContent = file.name;
                } else {
                    window._customFocusAudio = new Audio(dataUrl);
                    userData.focusSoundFileName = file.name;
                    if (focusFileName) focusFileName.textContent = file.name;
                }
                saveUserData();
                console.log(`[Sound] Successfully loaded custom ${category} sound: ${file.name}`);
            } catch (err) {
                console.error(`[Sound] Failed to save custom ${category} sound:`, err);
            }
        };
        reader.readAsDataURL(file);
    }

    focusFileInput?.addEventListener('change', () => handleCustomSoundFileSelect(focusFileInput, 'focus'));
    breakFileInput?.addEventListener('change', () => handleCustomSoundFileSelect(breakFileInput, 'break'));
    warningFileInput?.addEventListener('change', () => handleCustomSoundFileSelect(warningFileInput, 'warning'));

    // Sound Test Play Buttons
    document.getElementById('btn-play-focus-sound')?.addEventListener('click', () => {
        playNotificationSound('focus');
        sendNotification('Test Focus Sound', 'Testing your focus session notification sound.', 'focus');
    });

    document.getElementById('btn-play-break-sound')?.addEventListener('click', () => {
        playNotificationSound('break');
        sendNotification('Test Break Sound', 'Testing your break session notification sound.', 'break');
    });

    document.getElementById('btn-play-warning-sound')?.addEventListener('click', () => {
        playNotificationSound('warning');
        sendNotification('Test Warning Sound', 'Testing your 60-second warning notification sound.', 'warning');
    });

    // --- GIF Maintenance Utility ---
    document.getElementById('btn-cleanup-gifs')?.addEventListener('click', async () => {
        if (!confirm('This will remove all GIF history data except for your current active GIF. Continue?')) return;

        const activeId = userData.customGifData;
        const recentIds = (userData.recentGifs || []).map(g => g.data);
        const keepIds = new Set([activeId, ...recentIds, 'focus_sound', 'break_sound', 'warning_sound']);

        if (!gifDb._db) await gifDb.init();
        const tx = gifDb._db.transaction(STORE_NAME, 'readwrite');
        const store = tx.objectStore(STORE_NAME);
        const request = store.getAllKeys();

        request.onsuccess = () => {
            const keys = request.result;
            keys.forEach(key => {
                if (!keepIds.has(key)) {
                    store.delete(key);
                }
            });
            alert('Cleanup complete! Unused GIF data removed.');
            renderRecentGifs();
        };
    });

    btnCloseOptionsModal.addEventListener('click', () => {
        saveUserData();
        optionsModal.classList.remove('active');
    });

    // Save & Close Modal when clicking outside the modal content card (on the backdrop overlay)
    document.querySelectorAll('.modal-overlay').forEach(overlay => {
        overlay.addEventListener('mousedown', (e) => {
            if (e.target === overlay) {
                saveUserData();
                overlay.classList.remove('active');
            }
        });
    });

    // Save & Close Modal when pressing the Escape key
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            document.querySelectorAll('.modal-overlay.active').forEach(overlay => {
                saveUserData();
                overlay.classList.remove('active');
            });
        }

        // Prevent opening DevTools or browser inspection shortcuts in release mode
        const key = e.key ? e.key.toUpperCase() : '';
        const isDevShortcut =
            key === 'F12' ||
            (e.ctrlKey && e.shiftKey && (key === 'I' || key === 'J' || key === 'C')) ||
            (e.ctrlKey && (key === 'U' || key === 'R'));

        if (isDevShortcut && !window.__FOCUSGROW_DEV_MODE__) {
            e.preventDefault();
            e.stopPropagation();
        }
    });

    chkNotificationsToggle.addEventListener('change', (e) => {
        userData.notificationsEnabled = e.target.checked;
        saveUserData();
    });

    const chkAutoCloseToggle = document.getElementById('chk-auto-close-toggle');
    if (chkAutoCloseToggle) {
        chkAutoCloseToggle.checked = userData.autoCloseBlockedApps === true;
        chkAutoCloseToggle.addEventListener('change', (e) => {
            userData.autoCloseBlockedApps = e.target.checked;
            saveUserData();
            sendToCpp({
                action: 'setAutoCloseBlocked',
                enabled: userData.autoCloseBlockedApps
            });
            if (userData.autoCloseBlockedApps) {
                sendNotification('Auto-Close Enabled', 'Blocked apps will close automatically when opened during focus.', 'focus');
            }
        });
    }

    const chkBlockAllMonitorsToggle = document.getElementById('chk-block-all-monitors-toggle');
    if (chkBlockAllMonitorsToggle) {
        chkBlockAllMonitorsToggle.checked = userData.blockAllMonitors === true;
        // Sync setting to C++ immediately on startup
        sendToCpp({
            action: 'setBlockAllMonitors',
            enabled: userData.blockAllMonitors === true
        });
        chkBlockAllMonitorsToggle.addEventListener('change', (e) => {
            userData.blockAllMonitors = e.target.checked;
            saveUserData();
            sendToCpp({
                action: 'setBlockAllMonitors',
                enabled: userData.blockAllMonitors === true
            });
            if (userData.blockAllMonitors) {
                sendNotification('Multi-Monitor Block Enabled', 'Overlay will now cover all connected displays.', 'focus');
            } else {
                sendNotification('Single Monitor Block Enabled', 'Overlay will restrict the active display only.', 'focus');
            }
        });
    }

    const chkMinimizeTrayToggle = document.getElementById('chk-minimize-tray-toggle');
    if (chkMinimizeTrayToggle) {
        chkMinimizeTrayToggle.checked = userData.minimizeToTrayOnClose !== false;
        // Sync setting to C++ immediately on startup
        sendToCpp({
            action: 'setMinimizeToTrayConfig',
            enabled: userData.minimizeToTrayOnClose !== false
        });
        chkMinimizeTrayToggle.addEventListener('change', (e) => {
            userData.minimizeToTrayOnClose = e.target.checked;
            saveUserData();
            sendToCpp({
                action: 'setMinimizeToTrayConfig',
                enabled: userData.minimizeToTrayOnClose !== false
            });
        });
    }

    const chkAutoPauseToggle = document.getElementById('chk-auto-pause-toggle');
    const selectAutoPauseSec = document.getElementById('select-auto-pause-sec');
    const autoPauseSecRow = document.getElementById('auto-pause-sec-row');

    if (chkAutoPauseToggle) {
        chkAutoPauseToggle.checked = userData.autoPauseEnabled !== false;
        if (autoPauseSecRow) {
            autoPauseSecRow.style.opacity = (userData.autoPauseEnabled !== false) ? '1' : '0.5';
            autoPauseSecRow.style.pointerEvents = (userData.autoPauseEnabled !== false) ? 'auto' : 'none';
        }
        chkAutoPauseToggle.addEventListener('change', (e) => {
            userData.autoPauseEnabled = e.target.checked;
            if (autoPauseSecRow) {
                autoPauseSecRow.style.opacity = userData.autoPauseEnabled ? '1' : '0.5';
                autoPauseSecRow.style.pointerEvents = userData.autoPauseEnabled ? 'auto' : 'none';
            }
            saveUserData();
            sendToCpp({
                action: 'setAutoPauseConfig',
                enabled: userData.autoPauseEnabled,
                sec: userData.autoPauseSec || 15
            });
        });
    }

    if (selectAutoPauseSec) {
        selectAutoPauseSec.value = String(userData.autoPauseSec || 15);
        selectAutoPauseSec.addEventListener('change', (e) => {
            userData.autoPauseSec = parseInt(e.target.value, 10) || 15;
            saveUserData();
            sendToCpp({
                action: 'setAutoPauseConfig',
                enabled: userData.autoPauseEnabled !== false,
                sec: userData.autoPauseSec
            });
        });
    }

    function updateTimerSubtitle(data) {
        if (!upNextText) return;
        if (data) latestSessionData = data;
        const container = upNextContainer || upNextText.parentElement;
        const mode = userData.timerSubtitleMode || 'countdown';

        if (mode === 'hidden') {
            if (container) container.style.display = 'none';
            return;
        }

        if (container) container.style.display = 'flex';

        const state = (data && data.state) ? data.state : activeState;
        const curPeriod = (data && data.currentPeriod) ? data.currentPeriod : 1;
        const totPeriods = (data && data.totalPeriods) ? data.totalPeriods : 1;
        const remSec = (data && data.remainingSec !== undefined) ? data.remainingSec : 0;
        const skipBreaks = (data && data.skipBreaks !== undefined) ? !!data.skipBreaks : !!chkSkipBreaks.checked;
        const hasMoreBreaks = (curPeriod < totPeriods) && !skipBreaks;

        // Calculate total remaining session seconds across future periods
        const chunkMins = selectedPeriodMins || 25;
        let futureFocusSec = 0;
        for (let p = curPeriod + 1; p <= totPeriods; p++) {
            if (p === totPeriods) {
                let remMins = selectedMins % chunkMins;
                if (remMins === 0) remMins = chunkMins;
                futureFocusSec += remMins * 60;
            } else {
                futureFocusSec += chunkMins * 60;
            }
        }
        let futureBreaksCount = 0;
        if (!skipBreaks) {
            if (state === 'focusing') futureBreaksCount = Math.max(0, totPeriods - curPeriod);
            else if (state === 'resting') futureBreaksCount = Math.max(0, totPeriods - curPeriod - 1);
        }
        const futureBreakSec = futureBreaksCount * (selectedBreakMins || 5) * 60;
        const totalRemainingSec = remSec + futureFocusSec + futureBreakSec;

        if (mode === 'finishtime') {
            const finishDate = new Date(Date.now() + totalRemainingSec * 1000);
            const hh = String(finishDate.getHours()).padStart(2, '0');
            const mm = String(finishDate.getMinutes()).padStart(2, '0');
            const periodsLeft = Math.max(0, totPeriods - curPeriod);
            const periodSuffix = periodsLeft > 0 ? ` • ${periodsLeft} period${periodsLeft > 1 ? 's' : ''} left` : ' • Final period';
            upNextText.textContent = `Ends at ${hh}:${mm}${periodSuffix}`;
            upNextText.title = 'Estimated Finish Time (Click to switch display)';
        } else if (mode === 'totalrem') {
            const totalMins = Math.ceil(totalRemainingSec / 60);
            const h = Math.floor(totalMins / 60);
            const m = totalMins % 60;
            let durStr = '';
            if (h > 0 && m > 0) durStr = `${h}h ${m}m`;
            else if (h > 0) durStr = `${h}h`;
            else durStr = `${m}m`;
            upNextText.textContent = `${durStr} total remaining`;
            upNextText.title = 'Total Time Remaining (Click to switch display)';
        } else {
            // mode === 'countdown' (Default)
            if (container) container.style.display = 'flex';
            if (state === 'focusing') {
                if (hasMoreBreaks) {
                    // When <= 10 mins (600s): show countdown to upcoming break
                    // When > 10 mins: show calm 'Stay focused' so UI isn't empty/off
                    if (remSec <= 600) {
                        if (remSec <= 60 && remSec > 0) {
                            upNextText.textContent = `Break in ${remSec}s`;
                        } else {
                            const m = Math.ceil(remSec / 60);
                            upNextText.textContent = `Break in ${m} min${m > 1 ? 's' : ''}`;
                        }
                    } else {
                        upNextText.textContent = (data && data.isHealthMode) ? 'Posture Guard active' : 'Stay focused';
                    }
                } else if (skipBreaks && curPeriod < totPeriods) {
                    if (remSec <= 600) {
                        if (remSec <= 60 && remSec > 0) {
                            upNextText.textContent = `Next period in ${remSec}s`;
                        } else {
                            const m = Math.ceil(remSec / 60);
                            upNextText.textContent = `Next period in ${m} min${m > 1 ? 's' : ''}`;
                        }
                    } else {
                        upNextText.textContent = (data && data.isHealthMode) ? `Posture interval (${curPeriod}/${totPeriods})` : `Continuous focus (${curPeriod}/${totPeriods})`;
                    }
                } else {
                    if (remSec <= 600) {
                        const m = Math.ceil(remSec / 60);
                        upNextText.textContent = (remSec <= 60 && remSec > 0) ? `Final stretch (${remSec}s)` : `Final period (${m}m left)`;
                    } else {
                        upNextText.textContent = (data && data.isHealthMode) ? 'Final posture interval' : 'Final focus period';
                    }
                }
            } else if (state === 'resting') {
                if (remSec <= 60 && remSec > 0) {
                    upNextText.textContent = `Break ends in ${remSec}s`;
                } else {
                    const m = Math.ceil(remSec / 60);
                    upNextText.textContent = `Break ends in ${m} min${m > 1 ? 's' : ''}`;
                }
            } else {
                upNextText.textContent = 'Focus session';
            }
            upNextText.title = 'Break Countdown (Click to switch display)';
        }
    }

    if (upNextText) {
        upNextText.addEventListener('click', () => {
            const visibleCycle = ['countdown', 'finishtime', 'totalrem'];
            const currentMode = userData.timerSubtitleMode || 'countdown';
            let nextIndex = visibleCycle.indexOf(currentMode) + 1;
            if (nextIndex >= visibleCycle.length || nextIndex <= 0) nextIndex = 0;
            const newMode = visibleCycle[nextIndex];

            userData.timerSubtitleMode = newMode;
            saveUserData();

            if (selectTimerSubtitleMode) {
                selectTimerSubtitleMode.value = newMode;
            }
            if (latestSessionData) {
                updateTimerSubtitle(latestSessionData);
            }
        });
    }

    if (selectTimerSubtitleMode) {
        selectTimerSubtitleMode.value = userData.timerSubtitleMode || 'countdown';
        selectTimerSubtitleMode.addEventListener('change', (e) => {
            userData.timerSubtitleMode = e.target.value;
            saveUserData();
            if (latestSessionData) {
                updateTimerSubtitle(latestSessionData);
            }
        });
    }

    const chkNeutralOverstayToggle = document.getElementById('chk-neutral-overstay-toggle');
    const selectNeutralInterval = document.getElementById('select-neutral-overstay-interval');
    const chkNeutralSoftBlockToggle = document.getElementById('chk-neutral-softblock-toggle');

    if (chkNeutralOverstayToggle) {
        chkNeutralOverstayToggle.checked = userData.neutralOverstayEnabled !== false;
        chkNeutralOverstayToggle.addEventListener('change', (e) => {
            userData.neutralOverstayEnabled = e.target.checked;
            saveUserData();
        });
    }

    if (selectNeutralInterval) {
        selectNeutralInterval.value = String(userData.neutralOverstayIntervalMins || 30);
        selectNeutralInterval.addEventListener('change', (e) => {
            userData.neutralOverstayIntervalMins = parseInt(e.target.value, 10) || 30;
            saveUserData();
        });
    }

    if (chkNeutralSoftBlockToggle) {
        chkNeutralSoftBlockToggle.checked = userData.neutralSoftBlockEnabled !== false;
        chkNeutralSoftBlockToggle.addEventListener('change', (e) => {
            userData.neutralSoftBlockEnabled = e.target.checked;
            saveUserData();
        });
    }

    btnResetProgress.addEventListener('click', () => {
        userData.completedMinutesToday = 0;
        saveUserData();
        updateStatsUI();
        optionsModal.classList.remove('active');
    });

    btnClearWhitelist.addEventListener('click', () => {
        blockedApps = [];
        userData.blockedApps = blockedApps;
        saveUserData();
        sendToCpp({ action: 'setBlacklist', blacklist: blockedApps });
        renderAppList(cachedAppList);
        optionsModal.classList.remove('active');
    });

    // Filter & Sort State for Blocked Applications List
    let currentAppFilter = 'all'; // 'all', 'blocked', 'unblocked'
    let currentAppSort = 'latest'; // 'latest', 'name'

    // Filter Chips Listeners
    document.querySelectorAll('.filter-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            document.querySelectorAll('.filter-chip').forEach(c => c.classList.remove('active'));
            chip.classList.add('active');
            currentAppFilter = chip.getAttribute('data-filter') || 'all';
            renderAppList(cachedAppList);
        });
    });

    // Sort Dropdown Selector Listener
    document.getElementById('app-sort-select')?.addEventListener('change', (e) => {
        currentAppSort = e.target.value || 'latest';
        renderAppList(cachedAppList);
    });

    // Search Input Listener
    appSearchInput.addEventListener('input', () => {
        renderAppList(cachedAppList);
    });

    // Refresh Apps List
    btnRefreshApps.addEventListener('click', () => {
        sendToCpp({ action: 'getRunningApps' });
    });

    // Clean DOM Node App Whitelist Renderer
    window.renderAppList = function (apps) {
        if (apps) cachedAppList = apps;
        if (document.body.classList.contains('pip-mode')) {
            // RAM Optimization: When in PIP mode, only detect YT Music tabs without re-rendering invisible setup DOM
            detectYtMusicFromBrowserTabs(cachedAppList);
            return;
        }
        if (cachedAppList && Array.isArray(cachedAppList)) {
            userData.appIconMap = userData.appIconMap || {};
            cachedAppList.forEach(a => {
                if (a.exeName && a.icon && a.icon.length > 30) {
                    userData.appIconMap[a.exeName.toLowerCase()] = a.icon;
                }
            });
            saveUserData();
        }
        detectYtMusicFromBrowserTabs(cachedAppList);
        if (!appListContainer) return;
        appListContainer.innerHTML = '';

        const currentBlocked = userData.blockedApps || blockedApps || [];
        blockedApps = currentBlocked;

        // Combine currently open apps with any blocked apps (so blocked apps always show up in list)
        const knownExes = new Set((cachedAppList || []).map(a => (a.exeName || '').toLowerCase()));
        let combinedApps = [...(cachedAppList || [])];

        currentBlocked.forEach(bExe => {
            if (bExe && !knownExes.has(bExe.toLowerCase())) {
                const lower = bExe.toLowerCase();
                const icon = (userData.appIconMap && userData.appIconMap[lower]) || defaultIconSvg;
                combinedApps.push({
                    exeName: bExe,
                    title: bExe,
                    icon: icon
                });
            }
        });

        const searchQuery = (appSearchInput.value || '').trim().toLowerCase();

        let displayApps = combinedApps.filter(app => {
            const exeLower = (app.exeName || '').toLowerCase();
            if (exeLower === 'focusgrow.exe' || exeLower === 'msedgewebview2.exe') return false;

            const isBlocked = currentBlocked.some(a =>
                a.toLowerCase() === exeLower
            );
            if (currentAppFilter === 'blocked' && !isBlocked) return false;
            if (currentAppFilter === 'unblocked' && isBlocked) return false;

            if (!searchQuery) return true;
            return (app.title && app.title.toLowerCase().includes(searchQuery)) ||
                (app.exeName && app.exeName.toLowerCase().includes(searchQuery));
        });

        // Sort by Name (A-Z) or Latest Open
        if (currentAppSort === 'name') {
            displayApps.sort((a, b) => {
                const nameA = (a.title || a.exeName || '').toLowerCase();
                const nameB = (b.title || b.exeName || '').toLowerCase();
                return nameA.localeCompare(nameB);
            });
        }

        if (displayApps.length === 0) {
            appListContainer.innerHTML = `<div style="text-align:center; padding: 20px; font-size: 12px; color: var(--text-muted);">No matching applications found for filter: "${currentAppFilter}"</div>`;
            return;
        }

        displayApps.forEach(app => {
            const item = document.createElement('div');
            item.className = 'app-item';

            const iconImg = document.createElement('img');
            iconImg.className = 'app-icon';
            iconImg.src = app.icon && app.icon.length > 30 ? app.icon : defaultIconSvg;
            iconImg.onerror = () => { iconImg.src = defaultIconSvg; };

            const infoDiv = document.createElement('div');
            infoDiv.className = 'app-item-info';

            const titleDiv = document.createElement('div');
            titleDiv.className = 'app-item-title';
            titleDiv.textContent = app.title || app.exeName;

            const exeDiv = document.createElement('div');
            exeDiv.className = 'app-item-exe';
            exeDiv.textContent = app.exeName;

            infoDiv.appendChild(titleDiv);
            infoDiv.appendChild(exeDiv);

            const switchLabel = document.createElement('label');
            switchLabel.className = 'switch';

            const chk = document.createElement('input');
            chk.type = 'checkbox';
            chk.checked = currentBlocked.some(a =>
                a.toLowerCase() === app.exeName.toLowerCase()
            );

            const sliderSpan = document.createElement('span');
            sliderSpan.className = 'slider';

            switchLabel.appendChild(chk);
            switchLabel.appendChild(sliderSpan);

            chk.addEventListener('change', (e) => {
                let current = userData.blockedApps || blockedApps || [];
                if (e.target.checked) {
                    if (!current.some(a => a.toLowerCase() === app.exeName.toLowerCase())) {
                        current.push(app.exeName);
                    }
                } else {
                    current = current.filter(a =>
                        a.toLowerCase() !== app.exeName.toLowerCase()
                    );
                }
                userData.blockedApps = current;
                blockedApps = current;
                saveUserData();
                sendToCpp({ action: 'setBlacklist', blacklist: current });
                renderAppList(cachedAppList);
                if (typeof renderStatsDashboard === 'function') {
                    renderStatsDashboard(activeStatsRange);
                }
            });

            item.appendChild(iconImg);
            item.appendChild(infoDiv);
            item.appendChild(switchLabel);

            appListContainer.appendChild(item);
        });
    };

    // Daily Goal Multi-Tier Color Progression (Dual-Layer Clean Lap Rings)
    const GOAL_TIERS = [
        { name: 'Standard', color: 'rgba(255, 255, 255, 0.65)', baseColor: 'rgba(255, 255, 255, 0.1)' },
        { name: 'Electric Blue', color: '#38bdf8', baseColor: 'rgba(255, 255, 255, 0.4)' },
        { name: 'Emerald Green', color: '#22c55e', baseColor: '#38bdf8' },
        { name: 'Neon Pink', color: '#ec4899', baseColor: '#22c55e' },
        { name: 'Amber Gold', color: '#eab308', baseColor: '#ec4899' },
        { name: 'Crimson Flame', color: '#ef4444', baseColor: '#eab308' }
    ];

    function formatCompletedTimeDisplay(completedMins, formatMode = 'smart') {
        completedMins = Math.max(0, Number(completedMins) || 0);
        if (formatMode === 'decimal') {
            const hrs = (completedMins / 60).toFixed(1);
            return `${hrs} ${Number(hrs) === 1 ? 'hour' : 'hours'}`;
        }
        if (formatMode === 'minutes') {
            return `${completedMins} ${completedMins === 1 ? 'minute' : 'minutes'}`;
        }
        // Default 'smart' mode:
        if (completedMins < 60) {
            return `${completedMins} ${completedMins === 1 ? 'minute' : 'minutes'}`;
        }
        const h = Math.floor(completedMins / 60);
        const m = completedMins % 60;
        if (m === 0) {
            return `${h} ${h === 1 ? 'hour' : 'hours'}`;
        }
        return `${h}h ${m}m`;
    }

    function updateStatsUI() {
        userData.streakDays = calculateStreakDays();

        const yesterdayObj = new Date();
        yesterdayObj.setDate(new Date().getDate() - 1);
        const yDateStr = getLocalDateStr(yesterdayObj);
        const yMins = (userData.completedMinutesByDate && userData.completedMinutesByDate[yDateStr])
            ? userData.completedMinutesByDate[yDateStr]
            : 0;
        userData.yesterdayHours = (yMins / 60).toFixed(1);

        statGoalHours.textContent = userData.dailyGoalHours;
        statYesterday.textContent = userData.yesterdayHours;
        statStreak.textContent = userData.streakDays;
        if (statCompletedMins) statCompletedMins.textContent = userData.completedMinutesToday;

        const statCompletedDisplay = document.getElementById('stat-completed-display');
        if (statCompletedDisplay) {
            statCompletedDisplay.textContent = formatCompletedTimeDisplay(userData.completedMinutesToday, userData.completedTimeFormat || 'smart');
        }

        // Keep C++ FocusEngine LAN state server synchronized with latest streak and stats when changed
        syncStatsToCpp(false);

        const goalMins = Math.max(1, (Number(userData.dailyGoalHours) || 1) * 60);
        const completedMins = Math.max(0, Number(userData.completedMinutesToday) || 0);
        const totalRatio = completedMins / goalMins;
        const CIRCUMFERENCE = 301.5;

        if (goalDonutFill) {
            goalDonutFill.style.filter = 'none';

            if (totalRatio <= 0) {
                if (goalDonutBg) goalDonutBg.style.stroke = GOAL_TIERS[0].baseColor;
                goalDonutFill.style.stroke = GOAL_TIERS[0].color;
                goalDonutFill.style.strokeDashoffset = CIRCUMFERENCE;
                if (donutTierBadge) donutTierBadge.style.display = 'none';
                if (goalDonutContainer) goalDonutContainer.title = `Daily Goal: 0 / ${goalMins} mins (0%)`;
            } else if (totalRatio < 1.0) {
                if (goalDonutBg) goalDonutBg.style.stroke = GOAL_TIERS[0].baseColor;
                goalDonutFill.style.stroke = GOAL_TIERS[0].color;
                goalDonutFill.style.strokeDashoffset = (CIRCUMFERENCE * (1 - totalRatio)).toFixed(1);
                if (donutTierBadge) donutTierBadge.style.display = 'none';
                if (goalDonutContainer) goalDonutContainer.title = `Daily Goal: ${completedMins} / ${goalMins} mins (${Math.round(totalRatio * 100)}%)`;
            } else {
                const fullLaps = Math.floor(totalRatio);
                const lapProgress = totalRatio - fullLaps;
                const N = GOAL_TIERS.length - 1; // 5 colored tiers (Blue, Green, Pink, Yellow, Red)

                let activeTierIdx, fillRatio;
                if (lapProgress === 0) {
                    activeTierIdx = ((fullLaps - 1) % N) + 1;
                    fillRatio = 1.0;
                } else {
                    activeTierIdx = ((fullLaps - 1) % N) + 1;
                    fillRatio = lapProgress;
                }

                const currentTier = GOAL_TIERS[activeTierIdx] || GOAL_TIERS[1];

                if (goalDonutBg) goalDonutBg.style.stroke = currentTier.baseColor;
                goalDonutFill.style.stroke = currentTier.color;
                goalDonutFill.style.strokeDashoffset = (CIRCUMFERENCE * (1 - fillRatio)).toFixed(1);

                if (donutTierBadge) {
                    const multDisplay = totalRatio >= 10 ? totalRatio.toFixed(0) : totalRatio.toFixed(1);
                    donutTierBadge.style.display = 'inline-block';
                    donutTierBadge.style.color = currentTier.color;
                    donutTierBadge.style.borderColor = currentTier.color;
                    donutTierBadge.style.boxShadow = `0 2px 8px rgba(0, 0, 0, 0.6)`;
                    donutTierBadge.textContent = `${multDisplay}x`;
                }

                if (goalDonutContainer) {
                    const lapNumber = fullLaps + (lapProgress > 0 ? 1 : 0);
                    goalDonutContainer.title = `Daily Goal: ${completedMins} / ${goalMins} mins (${totalRatio.toFixed(1)}x - Lap ${lapNumber}: ${currentTier.name})`;
                }
            }
        }

        const statsKpiStreak = document.getElementById('stats-kpi-streak');
        if (statsKpiStreak) {
            statsKpiStreak.textContent = `${userData.streakDays || 0} Days`;
        }

        // Update Health Focus & Stand Streak Habit Stats
        userData.healthStreakDays = calculateHealthStreakDays();
        const statHealthBreaksToday = document.getElementById('stat-health-breaks-today');
        const statHealthGoalBreaks = document.getElementById('stat-health-goal-breaks');
        const statHealthStreak = document.getElementById('stat-health-streak');
        const statsKpiHealthStreak = document.getElementById('stats-kpi-health-streak');
        const statsKpiHealthSub = document.getElementById('stats-kpi-health-sub');

        const currentHealthBreaks = userData.healthCompletedBreaksToday || 0;
        const targetHealthBreaks = userData.healthDailyGoalBreaks || 4;
        const isShieldedToday = (currentHealthBreaks >= targetHealthBreaks && completedMins < goalMins);
        const isPostureShieldedByWork = (completedMins >= goalMins && currentHealthBreaks < targetHealthBreaks);

        const streakShieldIcon = document.getElementById('streak-shield-icon');
        const statStreakUnit = document.getElementById('stat-streak-unit');
        const healthShieldBadge = document.getElementById('health-shield-badge');
        const statsKpiStreakSub = document.getElementById('stats-kpi-streak-sub');

        if (streakShieldIcon) streakShieldIcon.style.display = isShieldedToday ? 'inline-block' : 'none';
        if (statStreakUnit) statStreakUnit.textContent = isShieldedToday ? 'days (shielded)' : 'days';
        if (healthShieldBadge) {
            if (currentHealthBreaks >= targetHealthBreaks) {
                healthShieldBadge.style.display = 'inline-block';
                healthShieldBadge.textContent = 'Shield Active';
            } else if (isPostureShieldedByWork) {
                healthShieldBadge.style.display = 'inline-block';
                healthShieldBadge.textContent = 'Protected by Work';
            } else {
                healthShieldBadge.style.display = 'none';
            }
        }
        if (statsKpiStreakSub) {
            statsKpiStreakSub.textContent = isShieldedToday ? 'Shielded via Posture Guard' : 'Productive work';
            statsKpiStreakSub.style.color = isShieldedToday ? '#34d399' : '';
        }

        if (statHealthBreaksToday) statHealthBreaksToday.textContent = currentHealthBreaks;
        if (statHealthGoalBreaks) statHealthGoalBreaks.textContent = targetHealthBreaks;
        if (statHealthStreak) statHealthStreak.textContent = userData.healthStreakDays || 0;
        if (statsKpiHealthStreak) statsKpiHealthStreak.textContent = `${userData.healthStreakDays || 0} Days`;
        if (statsKpiHealthSub) statsKpiHealthSub.textContent = `Posture habit (${currentHealthBreaks}/${targetHealthBreaks})`;

        if (statsModal && statsModal.classList.contains('active')) {
            renderWeeklyChart();
        }
    }

    // Interactive Completed Time Format Switcher (Hours & Mins ⇄ Decimal Hours ⇄ Minutes)
    const completedFooter = document.getElementById('completed-footer');
    if (completedFooter) {
        completedFooter.addEventListener('click', () => {
            const current = userData.completedTimeFormat || 'smart';
            let next = 'smart';
            if (current === 'smart') next = 'decimal';
            else if (current === 'decimal') next = 'minutes';
            else next = 'smart';

            userData.completedTimeFormat = next;
            saveUserData();
            updateStatsUI();
        });
    }

    // Statistics Modal Elements
    const statsModal = document.getElementById('stats-modal');
    const btnOpenStats = document.getElementById('btn-open-stats');
    const btnCloseStatsModal = document.getElementById('btn-close-stats-modal');

    // Quotes Database (Motivational & Wisdom Quotes in Indonesian & English)
    const FOCUS_QUOTES = [
        { text: "Tetap fokus pada tujuanmu. Hasil besar dibangun dari langkah-langkah kecil setiap hari.", author: "FocusGrow Wisdom" },
        { text: "Focus on being productive instead of busy.", author: "Tim Ferriss" },
        { text: "Konsistensi adalah kunci keberhasilan. Kerjakan tugasmu dengan penuh kesungguhan.", author: "Pepatah Produktif" },
        { text: "Deep work is the superpower of the 21st century.", author: "Cal Newport" },
        { text: "Jangan biarkan gangguan kecil menghalangi impian besarmu.", author: "Motivasi Kerja" },
        { text: "Do what you have to do until you can do what you want to do.", author: "Oprah Winfrey" },
        { text: "Satu jam fokus penuh jauh lebih berharga daripada seharian bekerja setengah hati.", author: "Prinsip Produktivitas" },
        { text: "Starve your distractions, feed your focus.", author: "Anonymous" },
        { text: "Kerjakan yang paling penting terlebih dahulu, biarkan yang lain menyusul.", author: "Manajemen Waktu" },
        { text: "Action is the foundational key to all success.", author: "Pablo Picasso" },
        { text: "Impian tidak terwujud lewat keajaiban; itu membutuhkan keringat, tekad, dan kerja keras.", author: "Colin Powell" },
        { text: "You don't need more time, you just need more focus.", author: "Productivity Master" },
        { text: "Fokus pada proses, hasil indah akan mengikuti dengan sendirinya.", author: "Filosofi Kerja" },
        { text: "It's not that I'm so smart, it's just that I stay with problems longer.", author: "Albert Einstein" },
        { text: "Fokus adalah seni mengatakan 'tidak' pada seribu hal baik lainnya.", author: "Steve Jobs" },
        { text: "Disiplin adalah jembatan antara cita-cita dan pencapaian.", author: "Success Logic" },
        { text: "Work hard in silence, let your success be your noise.", author: "Frank Ocean" },
        { text: "Cara terbaik untuk memulai adalah berhenti berbicara dan mulai melakukan.", author: "Walt Disney" },
        { text: "Success is not final, failure is not fatal: it is the courage to continue that counts.", author: "Winston Churchill" },
        { text: "Jangan menunggu waktu yang tepat. Ciptakan waktu itu sekarang juga.", author: "FocusGrow Wisdom" },
        { text: "Small steps in the right direction can turn out to be the biggest steps of your life.", author: "Daily Motivation" },
        { text: "Your mind is for having ideas, not holding them. Focus on the task at hand.", author: "David Allen" },
        { text: "Productivity is being able to do things that you were never able to do before.", author: "Franz Kafka" },
        { text: "Energi dan ketekunan menaklukkan segala hal.", author: "Benjamin Franklin" },
        { text: "Believe you can and you're halfway there.", author: "Theodore Roosevelt" }
    ];

    const REST_QUOTES = [
        { text: "Istirahat bukan berarti berhenti, melainkan mengisi ulang energi untuk melangkah lebih jauh.", author: "Nasihat Sehat" },
        { text: "Rest when you're weary. Refresh and renew yourself, your body, your mind, your spirit.", author: "Ralph Marston" },
        { text: "Jauhkan pandangan dari layar, regangkan tubuhmu, dan hirup udara segar.", author: "Panduan Istirahat" },
        { text: "Almost everything will work again if you unplug it for a few minutes, including you.", author: "Anne Lamott" },
        { text: "Kesehatan dan ketenangan pikiranmu adalah investasi terbaik untuk masa depan.", author: "Renungan Diri" },
        { text: "Take a break. A rested mind can solve problems that a tired mind cannot.", author: "Wellness Wisdom" },
        { text: "Minum air putih, berdiri sejenak, dan biarkan matamu beristirahat.", author: "Health Reminder" },
        { text: "Rest is not idleness; to lie on the grass under trees is by no means a waste of time.", author: "John Lubbock" },
        { text: "Tubuhmu butuh jeda agar bisa berlari kencang kembali nanti.", author: "FocusGrow Tips" },
        { text: "He who holds his breath for too long will collapse. Breathe and rest now.", author: "Ancient Wisdom" },
        { text: "Tidur adalah meditasi terbaik. Tapi saat ini, cukup regangkan badanmu saja.", author: "Dalai Lama" },
        { text: "Resting is a part of the process, not a reward for the process.", author: "Fitness Mental" },
        { text: "Sometimes the most productive thing you can do is relax.", author: "Mark Black" },
        { text: "Jangan merasa bersalah karena beristirahat. Mesin pun butuh pendinginan.", author: "Modern Productivity" },
        { text: "Tenangkan pikiranmu, dan jiwamu akan berbicara.", author: "Spirit Quote" },
        { text: "Your body hears everything your mind says. Give it some peace.", author: "Health First" },
        { text: "A change of pace is as good as a rest. Look away from the screen for a bit.", author: "Proverb" },
        { text: "The time to relax is when you don't have time for it.", author: "Sydney J. Harris" },
        { text: "Refresh your mind, clear your vision, and recharge your soul.", author: "Zen Master" },
        { text: "Breathe in confidence, breathe out doubt. Take this moment for yourself.", author: "Mindfulness" }
    ];

    function updateRandomQuote(mode = 'focus') {
        const quotes = (mode === 'rest' || mode === 'resting') ? REST_QUOTES : FOCUS_QUOTES;
        const q = quotes[Math.floor(Math.random() * quotes.length)];

        const qTextSetup = document.getElementById('quote-text-setup');
        const qAuthorSetup = document.getElementById('quote-author-setup');
        const qTextTimer = document.getElementById('quote-text-timer');
        const qAuthorTimer = document.getElementById('quote-author-timer');

        if (qTextSetup) qTextSetup.textContent = `"${q.text}"`;
        if (qAuthorSetup) qAuthorSetup.textContent = `— ${q.author}`;
        if (qTextTimer) qTextTimer.textContent = `"${q.text}"`;
        if (qAuthorTimer) qAuthorTimer.textContent = `— ${q.author}`;
    }

    updateRandomQuote('focus');

    // Render Statistics Dashboard Modal
    let activeStatsRange = 'today';

    // Helper: Format Seconds to Human readable (e.g. 1h 24m or 45m)
    function formatSecs(secs) {
        if (!secs || secs <= 0) return '0m';
        const h = Math.floor(secs / 3600);
        const m = Math.floor((secs % 3600) / 60);
        if (h > 0) return `${h}h ${m}m`;
        return `${m}m`;
    }

    function formatLiveStreak(sec, limitMins) {
        if (sec < 60) return `⚡ ${sec}s / ${limitMins}m`;
        const m = Math.floor(sec / 60);
        const s = sec % 60;
        return (s > 0) ? `⚡ ${m}m ${s}s / ${limitMins}m` : `⚡ ${m}m / ${limitMins}m`;
    }

    // Helper: Get Date Strings Array for Today, Week (7 days), Month (30 days), or Quarter (90 days)
    function getDateRangeArray(range) {
        const dates = [];
        const days = range === 'quarter' ? 90 : (range === 'month' ? 30 : (range === 'week' ? 7 : 1));
        const now = new Date();
        for (let i = 0; i < days; i++) {
            const d = new Date(now);
            d.setDate(d.getDate() - i);
            dates.push(getLocalDateStr(d));
        }
        return dates;
    }

    // Helper: Formats human-friendly month range (e.g., "Jul — Sep 2026" or "Aug — Sep 2026")
    function getPeriodMonthSpanText(days) {
        const now = new Date();
        const startD = new Date(now);
        startD.setDate(startD.getDate() - (days - 1));
        const sMonth = startD.toLocaleDateString('en-US', { month: 'short' });
        const eMonth = now.toLocaleDateString('en-US', { month: 'short' });
        const sYear = startD.getFullYear();
        const eYear = now.getFullYear();

        if (days === 1) {
            return now.toLocaleDateString('en-US', { day: 'numeric', month: 'short', year: 'numeric' });
        }
        if (sYear === eYear) {
            if (sMonth === eMonth) {
                return `${sMonth} ${eYear}`;
            }
            return `${sMonth} — ${eMonth} ${eYear}`;
        }
        return `${sMonth} ${sYear} — ${eMonth} ${eYear}`;
    }

    // Statistics Modal Handlers
    document.querySelectorAll('#btn-open-stats, .btn-stats-trigger').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            document.querySelectorAll('.modal-overlay.active').forEach(m => m.classList.remove('active'));
            const targetModal = document.getElementById('stats-modal');
            if (targetModal) {
                targetModal.classList.add('active');
                renderStatsDashboard('today');
            }
        });
    });

    btnCloseStatsModal?.addEventListener('click', () => {
        const targetModal = document.getElementById('stats-modal');
        if (targetModal) targetModal.classList.remove('active');
    });

    document.querySelectorAll('.stats-tab-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const range = btn.getAttribute('data-range');
            if (range) {
                renderStatsDashboard(range);
            }
        });
    });

    // Default classification for popular apps
    const DEFAULT_APP_CATEGORIES = {
        'afterfx.exe': 'productive',
        'premiere.exe': 'productive',
        'photoshop.exe': 'productive',
        'illustrator.exe': 'productive',
        'blender.exe': 'productive',
        'figma.exe': 'productive',
        'canva.exe': 'productive',
        'audition.exe': 'productive',
        'obs64.exe': 'productive',
        'studio64.exe': 'productive',
        'code.exe': 'productive',
        'devenv.exe': 'productive',
        'rider64.exe': 'productive',
        'idea64.exe': 'productive',
        'pycharm64.exe': 'productive',
        'clion64.exe': 'productive',
        'webstorm64.exe': 'productive',
        'unity.exe': 'productive',
        'unrealeditor.exe': 'productive',
        'sublime_text.exe': 'productive',
        'cursor.exe': 'productive',
        'zed.exe': 'productive',
        'resolve.exe': 'productive',
        'antigravity ide.exe': 'productive',
        'antigravity.exe': 'productive',
        'git-bash.exe': 'productive',
        'windowsterminal.exe': 'productive',
        'powershell.exe': 'productive',
        'cmd.exe': 'productive',
        'word.exe': 'productive',
        'excel.exe': 'productive',
        'powerpnt.exe': 'productive',
        'notion.exe': 'productive',
        'obsidian.exe': 'productive',
        'steam.exe': 'distracting',
        'epicgameslauncher.exe': 'distracting',
        'discord.exe': 'distracting',
        'spotify.exe': 'distracting',
        'telegram.exe': 'distracting',
        'whatsapp.exe': 'distracting',
        'tiktok.exe': 'distracting'
    };

    function getAppCategory(exeName) {
        if (!exeName) return 'neutral';
        const lower = exeName.toLowerCase();
        userData.appCategories = userData.appCategories || {};
        if (userData.appCategories[lower]) {
            return userData.appCategories[lower];
        }
        if (DEFAULT_APP_CATEGORIES[lower]) {
            return DEFAULT_APP_CATEGORIES[lower];
        }
        return 'neutral';
    }

    function toggleAppCategory(exeName) {
        if (!exeName) return;
        const lower = exeName.toLowerCase();
        const current = getAppCategory(exeName);
        const next = current === 'productive' ? 'neutral' : (current === 'neutral' ? 'distracting' : 'productive');
        userData.appCategories = userData.appCategories || {};
        userData.appCategories[lower] = next;
        saveUserData();
        renderStatsDashboard(activeStatsRange);
    }

    function toggleQuickBlockApp(exeName) {
        if (!exeName) return;
        let blocked = userData.blockedApps || [];
        const lower = exeName.toLowerCase();
        const idx = blocked.findIndex(a => a.toLowerCase() === lower);
        if (idx >= 0) {
            blocked.splice(idx, 1);
            sendNotification('App Unblocked', `${exeName} removed from restricted apps.`, 'focus');
        } else {
            blocked.push(exeName);
            sendNotification('App Blocked', `${exeName} added to restricted apps list.`, 'focus');
        }
        userData.blockedApps = blocked;
        blockedApps = blocked;
        saveUserData();
        sendToCpp({ action: 'setBlacklist', blacklist: blocked });
        renderStatsDashboard(activeStatsRange);
        if (typeof window.renderAppList === 'function') {
            window.renderAppList(cachedAppList);
        }
    }

    function renderWeeklyChart() {
        const chartContainer = document.getElementById('stats-weekly-chart');
        if (!chartContainer) return;

        chartContainer.innerHTML = '';
        const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
        const today = new Date();

        // Target daily goal in mins
        const dailyGoalHours = parseFloat(userData.dailyGoalHours) || 1;
        const dailyGoalMins = dailyGoalHours * 60;

        // Gather 7 days data first to determine dynamic scale
        const daysData = [];
        let maxMins = dailyGoalMins;

        for (let i = 6; i >= 0; i--) {
            const d = new Date();
            d.setDate(today.getDate() - i);
            const dateStr = getLocalDateStr(d);
            const dayName = dayNames[d.getDay()];

            let mins = (dateStr === todayDateStr) ? (userData.completedMinutesToday || 0) : ((userData.completedMinutesByDate && userData.completedMinutesByDate[dateStr]) || 0);

            if (mins === 0 && userData.appStatsByDate && userData.appStatsByDate[dateStr]) {
                const dayApps = userData.appStatsByDate[dateStr];
                let totalSec = 0;
                for (const exe in dayApps) {
                    totalSec += dayApps[exe];
                }
                mins = Math.floor(totalSec / 60);
            }

            if (mins > maxMins) {
                maxMins = mins;
            }

            daysData.push({
                index: i,
                dateStr,
                dayName: (i === 0 ? 'Today' : dayName),
                mins
            });
        }

        if (maxMins <= 0) maxMins = 60;

        daysData.forEach(item => {
            const ratio = item.mins / maxMins;
            const barWrapper = document.createElement('div');
            barWrapper.className = 'chart-bar-wrapper';

            const displayPercent = item.mins > 0 ? Math.max(6, Math.round(ratio * 100)) : 0;
            const isGoalMet = isGoalMetWithTolerance(item.mins, dailyGoalMins, dailyGoalHours);
            const hoursPart = Math.floor(item.mins / 60);
            const minsPart = item.mins % 60;
            const timeFormattedStr = `${hoursPart}h ${minsPart}m`;

            barWrapper.innerHTML = `
                <div class="chart-tooltip">
                    <div class="chart-tooltip-title">${item.dayName} (${item.dateStr})</div>
                    <div class="chart-tooltip-time">⏱️ ${timeFormattedStr} (${item.mins}m)</div>
                    <div class="chart-tooltip-status">${isGoalMet ? '🎉 Goal Met!' : '⏳ Target: ' + dailyGoalHours + 'h'}</div>
                </div>
                <div class="chart-bar-container">
                    <div class="chart-bar-fill ${isGoalMet ? 'goal-met' : ''}" style="height: ${displayPercent}%;">
                        ${item.mins >= (maxMins * 0.12) ? `<span class="bar-val-hint">${item.mins}m</span>` : ''}
                    </div>
                </div>
                <div class="chart-day-label">${item.dayName}</div>
            `;
            chartContainer.appendChild(barWrapper);
        });
    }

    let heatmapWeekOffset = 0;

    function renderHeatmapMatrix() {
        const matrix = document.getElementById('stats-github-heatmap-matrix');
        const monthsBar = document.getElementById('github-heatmap-months');
        const totalText = document.getElementById('heatmap-total-contributions-text');
        const periodLabel = document.getElementById('heatmap-period-label');
        if (!matrix || !monthsBar) return;
        matrix.innerHTML = '';
        monthsBar.innerHTML = '';

        const WEEKS_TO_SHOW = 34; // 34 weeks distributed with justify-content: space-between fills 100% width
        const dailyGoalMins = (userData.dailyGoalHours || 1) * 60;

        // End date adjusted by heatmapWeekOffset
        const endDate = new Date();
        endDate.setDate(endDate.getDate() + (heatmapWeekOffset * 7));

        // Find the Saturday of that week (end of week column)
        const endDayOfWeek = endDate.getDay();
        const currentSaturday = new Date(endDate);
        currentSaturday.setDate(currentSaturday.getDate() + (6 - endDayOfWeek));

        // Start date: (WEEKS_TO_SHOW * 7 - 1) days before currentSaturday
        const totalDays = WEEKS_TO_SHOW * 7;
        const startDate = new Date(currentSaturday);
        startDate.setDate(startDate.getDate() - (totalDays - 1));

        let totalPeriodMins = 0;
        let activeDaysCount = 0;
        const monthPositions = [];
        let lastMonth = -1;

        // Render columns (weeks) and rows (days 0..6: Sun..Sat)
        for (let col = 0; col < WEEKS_TO_SHOW; col++) {
            const weekCol = document.createElement('div');
            weekCol.className = 'github-week-col';

            for (let row = 0; row < 7; row++) {
                const dayIndex = col * 7 + row;
                const cellDate = new Date(startDate);
                cellDate.setDate(cellDate.getDate() + dayIndex);

                const dateStr = getLocalDateStr(cellDate);
                const dayName = cellDate.toLocaleDateString('en-US', { weekday: 'short' });
                const monthName = cellDate.toLocaleDateString('en-US', { month: 'short' });
                const monthNum = cellDate.getMonth();

                // Detect month header boundary
                if (row === 0 && monthNum !== lastMonth) {
                    monthPositions.push({ name: monthName, colIndex: col });
                    lastMonth = monthNum;
                }

                let mins = (dateStr === todayDateStr) ? (userData.completedMinutesToday || 0) : ((userData.completedMinutesByDate && userData.completedMinutesByDate[dateStr]) || 0);

                if (mins === 0 && userData.appStatsByDate && userData.appStatsByDate[dateStr]) {
                    let totalSec = 0;
                    for (const exe in userData.appStatsByDate[dateStr]) {
                        totalSec += userData.appStatsByDate[dateStr][exe];
                    }
                    mins = Math.floor(totalSec / 60);
                }

                totalPeriodMins += mins;
                if (mins > 0) activeDaysCount++;

                let lvl = 'lvl-0';
                if (mins > 0) {
                    if (mins >= dailyGoalMins || mins >= 120) lvl = 'lvl-4';
                    else if (mins >= 60) lvl = 'lvl-3';
                    else if (mins >= 30) lvl = 'lvl-2';
                    else lvl = 'lvl-1';
                }

                const cell = document.createElement('div');
                cell.className = `github-cell ${lvl}`;
                const hoursPart = Math.floor(mins / 60);
                const minsPart = mins % 60;
                const formattedTime = (hoursPart > 0 ? `${hoursPart}h ` : '') + `${minsPart}m`;

                cell.title = `${dateStr} (${dayName}): ${mins > 0 ? formattedTime + ' focused' : 'No focus recorded'}`;
                weekCol.appendChild(cell);
            }
            matrix.appendChild(weekCol);
        }

        // Render Month Headers aligned with columns percentage
        monthPositions.forEach(m => {
            const span = document.createElement('span');
            span.className = 'github-month-label';
            span.textContent = m.name;
            const pct = (m.colIndex / (WEEKS_TO_SHOW - 1)) * 96;
            span.style.left = `${pct}%`;
            monthsBar.appendChild(span);
        });

        // Update header summary text
        const totalHours = (totalPeriodMins / 60).toFixed(1);
        if (totalText) {
            totalText.textContent = `${totalHours} hours focused in ${WEEKS_TO_SHOW} weeks (${activeDaysCount} active days)`;
        }

        // Update period label
        if (periodLabel) {
            const startMonthStr = startDate.toLocaleDateString('en-US', { month: 'short', year: 'numeric' });
            const endMonthStr = currentSaturday.toLocaleDateString('en-US', { month: 'short', year: 'numeric' });
            periodLabel.textContent = `${startMonthStr} — ${endMonthStr}`;
        }
    }

    document.getElementById('btn-heatmap-prev')?.addEventListener('click', (e) => {
        e.stopPropagation();
        heatmapWeekOffset -= 12; // Go back ~3 months
        renderHeatmapMatrix();
    });

    document.getElementById('btn-heatmap-next')?.addEventListener('click', (e) => {
        e.stopPropagation();
        heatmapWeekOffset = Math.min(0, heatmapWeekOffset + 12); // Advance towards today
        renderHeatmapMatrix();
    });

    // Intelligent Chronotype Evaluator (handles All-Day Marathon Titans, Early Birds, Afternoon Masters, Night Owls)
    function evaluateChronotype(hourlyBuckets, peakHour, totalMins, rangeInfo) {
        if (!totalMins || totalMins <= 0) {
            return {
                persona: '⏳ Awaiting Data',
                color: 'var(--text-muted)',
                desc: 'Start focus sessions throughout the day to calculate your peak productivity curve.'
            };
        }

        const morningMins = hourlyBuckets.slice(5, 12).reduce((a, b) => a + b, 0);
        const afternoonMins = hourlyBuckets.slice(12, 18).reduce((a, b) => a + b, 0);
        const nightMins = hourlyBuckets.slice(18, 24).reduce((a, b) => a + b, 0);
        const midnightMins = hourlyBuckets.slice(0, 5).reduce((a, b) => a + b, 0);
        const effectiveTotal = Math.max(1, morningMins + afternoonMins + nightMins + midnightMins);

        // Detect active work span (e.g. from morning to late evening)
        let firstH = -1, lastH = -1;
        for (let h = 0; h < 24; h++) {
            if (hourlyBuckets[h] > 5) {
                if (firstH === -1) firstH = h;
                lastH = h;
            }
        }
        if (rangeInfo && rangeInfo.firstStart && rangeInfo.lastActive) {
            const parseH = (tStr) => {
                const match = tStr.match(/(\d+):(\d+)\s*(AM|PM)?/i);
                if (match) {
                    let h = parseInt(match[1], 10);
                    const isPM = (match[3] || '').toUpperCase() === 'PM';
                    const isAM = (match[3] || '').toUpperCase() === 'AM';
                    if (isPM && h < 12) h += 12;
                    if (isAM && h === 12) h = 0;
                    return h;
                }
                return -1;
            };
            const rStart = parseH(rangeInfo.firstStart);
            const rEnd = parseH(rangeInfo.lastActive);
            if (rStart >= 0 && (firstH === -1 || rStart < firstH)) firstH = rStart;
            if (rEnd >= 0 && (lastH === -1 || rEnd > lastH)) lastH = rEnd;
        }

        const spanHours = (firstH >= 0 && lastH >= 0 && lastH >= firstH) ? (lastH - firstH + 1) : 0;
        const hasSolidMorning = morningMins >= 25 || (morningMins / effectiveTotal) >= 0.15;
        const hasSolidAfternoon = afternoonMins >= 35 || (afternoonMins / effectiveTotal) >= 0.20;
        const hasSolidNight = nightMins >= 35 || (nightMins / effectiveTotal) >= 0.20;

        // If span is >= 8 hours (morning to night) AND has active work across morning and afternoon/night:
        const isAllDayTitan = (spanHours >= 8 && hasSolidMorning && (hasSolidAfternoon || hasSolidNight)) ||
                              (totalMins >= 240 && hasSolidMorning && (hasSolidAfternoon || hasSolidNight));

        if (isAllDayTitan) {
            return {
                persona: '⚡ All-Day Focus Titan',
                color: '#fbbf24',
                desc: `Marathon deep worker maintaining sustained endurance from morning into evening, with peak momentum around ${String(peakHour).padStart(2, '0')}:00 — ${String((peakHour + 2) % 24).padStart(2, '0')}:00.`
            };
        } else if (peakHour >= 5 && peakHour < 12) {
            return {
                persona: '🌅 Early Bird Champion',
                color: '#fbbf24',
                desc: `Prime focus peaks during fresh morning hours (${String(peakHour).padStart(2, '0')}:00 — ${String((peakHour + 2) % 24).padStart(2, '0')}:00).`
            };
        } else if (peakHour >= 12 && peakHour < 18) {
            return {
                persona: '⚡ Afternoon Flow Master',
                color: '#60cdff',
                desc: `Unlocks strongest deep work momentum during afternoon hours (${String(peakHour).padStart(2, '0')}:00 — ${String((peakHour + 2) % 24).padStart(2, '0')}:00).`
            };
        } else if (peakHour >= 18 && peakHour <= 23) {
            return {
                persona: '🌙 Night Owl Coder',
                color: '#c084fc',
                desc: `Achieves effortless flow state in late evenings and night hours (${String(peakHour).padStart(2, '0')}:00 — ${String((peakHour + 2) % 24).padStart(2, '0')}:00).`
            };
        } else {
            return {
                persona: '🌌 Midnight Hacker',
                color: '#a78bfa',
                desc: `Late night specialist thriving in zero-distraction quiet hours (${String(peakHour).padStart(2, '0')}:00 — ${String((peakHour + 2) % 24).padStart(2, '0')}:00).`
            };
        }
    }

    function renderPeakHoursChart(range = activeStatsRange) {
        const svg = document.getElementById('peak-wave-svg');
        const areaPath = document.getElementById('peak-wave-area');
        const linePath = document.getElementById('peak-wave-line');
        const peakMarker = document.getElementById('stats-peak-marker');
        const peakDot = peakMarker ? peakMarker.querySelector('.peak-marker-dot') : null;
        const peakPulse = peakMarker ? peakMarker.querySelector('.peak-marker-pulse') : null;
        const hoverLine = document.getElementById('wave-hover-line');
        const hoverDot = document.getElementById('stats-wave-hover-dot');
        const hoverTooltip = document.getElementById('wave-hover-tooltip');
        const waveContainer = document.getElementById('stats-peak-wave-container');
        const badge = document.getElementById('stats-chronotype-badge');
        const insight = document.getElementById('stats-peak-hours-insight');
        if (!areaPath || !linePath) return;

        const targetDates = getDateRangeArray(range);
        const hourlyBuckets = new Array(24).fill(0);

        // Aggregate hourly focus time
        targetDates.forEach(dateStr => {
            if (userData.hourlyFocusStats && userData.hourlyFocusStats[dateStr]) {
                const dayHours = userData.hourlyFocusStats[dateStr];
                for (let h = 0; h < 24; h++) {
                    if (dayHours[h]) hourlyBuckets[h] += dayHours[h];
                }
            } else {
                let dayMins = (dateStr === todayDateStr) ? (userData.completedMinutesToday || 0) : ((userData.completedMinutesByDate && userData.completedMinutesByDate[dateStr]) || 0);
                if (dayMins > 0) {
                    const rangeInfo = (userData.workTimeRangesByDate && userData.workTimeRangesByDate[dateStr]);
                    let startH = 9, endH = 18;
                    if (rangeInfo && rangeInfo.firstStart && rangeInfo.lastActive) {
                        const parseH = (tStr) => {
                            const match = tStr.match(/(\d+):(\d+)\s*(AM|PM)?/i);
                            if (match) {
                                let h = parseInt(match[1], 10);
                                const isPM = (match[3] || '').toUpperCase() === 'PM';
                                const isAM = (match[3] || '').toUpperCase() === 'AM';
                                if (isPM && h < 12) h += 12;
                                if (isAM && h === 12) h = 0;
                                return h;
                            }
                            return 9;
                        };
                        startH = parseH(rangeInfo.firstStart);
                        endH = parseH(rangeInfo.lastActive);
                        if (endH < startH) endH = Math.min(23, startH + 4);
                    }
                    const numHours = Math.max(1, (endH - startH + 1));
                    const baseMins = Math.floor(dayMins / numHours);
                    let rem = dayMins % numHours;
                    for (let h = startH; h <= endH; h++) {
                        const bonus = rem > 0 ? 1 : 0;
                        if (rem > 0) rem--;
                        hourlyBuckets[h] += (baseMins + bonus);
                    }
                }
            }
        });

        let maxH = Math.max(1, ...hourlyBuckets);
        let peakWindowStart = 0;
        let peakWindowSum = 0;
        let highestSingleHour = 0;
        let highestSingleVal = 0;

        for (let h = 0; h < 24; h++) {
            const twoHourSum = hourlyBuckets[h] + hourlyBuckets[(h + 1) % 24];
            if (twoHourSum > peakWindowSum) {
                peakWindowSum = twoHourSum;
                peakWindowStart = h;
            }
            if (hourlyBuckets[h] > highestSingleVal) {
                highestSingleVal = hourlyBuckets[h];
                highestSingleHour = h;
            }
        }

        let peakHour = peakWindowStart;
        let peakWindow = `${String(peakHour).padStart(2, '0')}:00 — ${String((peakHour + 2) % 24).padStart(2, '0')}:00`;

        // Chronotype classification with marathon all-day intelligence
        const totalAggMins = hourlyBuckets.reduce((a, b) => a + b, 0);
        const todayRangeInfo = (userData.workTimeRangesByDate && userData.workTimeRangesByDate[todayDateStr]);
        const chronoInfo = evaluateChronotype(hourlyBuckets, peakHour, totalAggMins, todayRangeInfo);

        if (badge) {
            if (totalAggMins > 0) {
                badge.textContent = chronoInfo.persona;
                badge.style.color = chronoInfo.color;
            } else {
                badge.innerHTML = '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 3px;"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg> Awaiting Data';
                badge.style.color = 'var(--text-muted)';
            }
        }

        if (insight) {
            if (peakWindowSum > 0 && totalAggMins > 0) {
                const peakHPart = Math.floor(peakWindowSum / 60);
                const peakMPart = peakWindowSum % 60;
                const formattedPeak = (peakHPart > 0 ? `${peakHPart}h ` : '') + `${peakMPart}m`;
                insight.innerHTML = `<span class="insight-icon-svg"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#f59e0b" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z"/></svg></span>Your peak concentration flow is around <strong>${peakWindow}</strong> (${formattedPeak} focused). ${chronoInfo.desc}`;
            } else {
                insight.innerHTML = `<span class="insight-icon-svg"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#fbbf24" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="9" y1="18" x2="15" y2="18"></line><line x1="10" y1="22" x2="14" y2="22"></line><path d="M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0 0 18 8 6 6 0 0 0 6 8c0 1 .23 2.23 1.5 3.5A4.61 4.61 0 0 1 8.91 14"></path></svg></span>Start focus sessions throughout the day to calculate your peak productivity curve.`;
            }
        }

        // Calculate 24 points for the SVG (viewBox 0 0 500 100)
        // Baseline Y = 88, Peak Y = 16
        const points = [];
        for (let h = 0; h < 24; h++) {
            const x = (h / 23) * 500;
            const val = hourlyBuckets[h];
            const y = (maxH > 0 && val > 0) ? (88 - ((val / maxH) * 70)) : 88;
            points.push({ x, y, val, hour: h });
        }

        // Generate Monotone Spline Path
        let lineD = `M ${points[0].x.toFixed(1)} ${points[0].y.toFixed(1)}`;
        for (let i = 0; i < points.length - 1; i++) {
            const p0 = points[i === 0 ? 0 : i - 1];
            const p1 = points[i];
            const p2 = points[i + 1];
            const p3 = points[i + 2 >= points.length ? points.length - 1 : i + 2];

            const cp1x = p1.x + (p2.x - p0.x) / 5;
            const cp1y = p1.y + (p2.y - p0.y) / 5;
            const cp2x = p2.x - (p3.x - p1.x) / 5;
            const cp2y = p2.y - (p3.y - p1.y) / 5;

            lineD += ` C ${cp1x.toFixed(1)} ${cp1y.toFixed(1)}, ${cp2x.toFixed(1)} ${cp2y.toFixed(1)}, ${p2.x.toFixed(1)} ${p2.y.toFixed(1)}`;
        }

        const areaD = lineD + ` L 500 88 L 0 88 Z`;
        linePath.setAttribute('d', lineD);
        areaPath.setAttribute('d', areaD);

        // Position Peak Pulse Marker (100% Round HTML Overlay, Never Gepeng)
        if (peakMarker && highestSingleVal > 0) {
            const peakPt = points[highestSingleHour];
            peakMarker.style.left = `${(peakPt.x / 500) * 100}%`;
            peakMarker.style.top = `${(peakPt.y / 100) * 100}%`;
            peakMarker.style.display = 'flex';
            if (peakDot) {
                peakDot.style.background = chronoInfo.color || '#60cdff';
                peakDot.style.boxShadow = `0 0 10px ${chronoInfo.color || '#60cdff'}, 0 0 4px #ffffff`;
            }
            if (peakPulse) {
                peakPulse.style.background = (chronoInfo.color || '#60cdff') + '44';
            }
        } else if (peakMarker) {
            peakMarker.style.display = 'none';
        }

        // Interactive Tooltip & Crosshair on Hover
        if (waveContainer && hoverLine && hoverDot && hoverTooltip) {
            waveContainer.onmousemove = (e) => {
                const rect = waveContainer.getBoundingClientRect();
                const mouseX = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
                const ratio = mouseX / rect.width;
                const hourIndex = Math.round(ratio * 23);
                const pt = points[hourIndex];

                const svgX = (hourIndex / 23) * 500;
                hoverLine.setAttribute('x1', svgX);
                hoverLine.setAttribute('x2', svgX);
                hoverLine.style.display = 'block';

                hoverDot.style.left = `${(svgX / 500) * 100}%`;
                hoverDot.style.top = `${(pt.y / 100) * 100}%`;
                hoverDot.style.display = 'block';

                hoverTooltip.textContent = `${String(pt.hour).padStart(2, '0')}:00 — ${pt.val} min focused`;
                hoverTooltip.style.left = `${(svgX / 500) * rect.width}px`;
                hoverTooltip.classList.add('show');
            };

            waveContainer.onmouseleave = () => {
                hoverLine.style.display = 'none';
                hoverDot.style.display = 'none';
                hoverTooltip.classList.remove('show');
            };
        }
    }

    function renderStatsDashboard(range = activeStatsRange) {
        activeStatsRange = range;

        document.querySelectorAll('.stats-tab-btn[data-range]').forEach(btn => {
            btn.classList.toggle('active', btn.getAttribute('data-range') === range);
        });

        const targetDates = getDateRangeArray(range);

        let totalSecs = 0;
        const appMap = {};
        userData.appStatsByDate = userData.appStatsByDate || {};
        userData.workTimeRangesByDate = userData.workTimeRangesByDate || {};

        targetDates.forEach(dateStr => {
            const dayStats = userData.appStatsByDate[dateStr];
            if (dayStats) {
                Object.keys(dayStats).forEach(exe => {
                    const sec = dayStats[exe] || 0;
                    totalSecs += sec;
                    appMap[exe] = (appMap[exe] || 0) + sec;
                });
            }
        });

        // Summary Cards
        document.getElementById('stats-kpi-total-time').textContent = formatSecs(totalSecs);

        const totalMins = Math.floor(totalSecs / 60);
        const daysCount = (range === 'quarter' ? 90 : (range === 'month' ? 30 : (range === 'week' ? 7 : 1)));
        const targetGoalMins = (userData.dailyGoalHours || 1) * 60 * daysCount;
        const goalPercent = Math.min(100, Math.round((totalMins / targetGoalMins) * 100));

        const spanText = getPeriodMonthSpanText(daysCount);
        const shortSpan = spanText.replace(' ' + new Date().getFullYear(), '');

        document.getElementById('stats-kpi-goal-percent').textContent = `${goalPercent}%`;
        document.getElementById('stats-kpi-goal-sub').textContent = (range === 'today') 
            ? `Target: ${(targetGoalMins / 60).toFixed(0)} hr` 
            : `Target: ${(targetGoalMins / 60).toFixed(0)} hrs (${shortSpan})`;
        document.getElementById('stats-kpi-streak').textContent = `${userData.streakDays || 0} Days`;

        const flowPeriodLabel = document.getElementById('stats-flow-period-label');
        if (flowPeriodLabel) {
            if (range === 'quarter') flowPeriodLabel.textContent = `• ${spanText}`;
            else if (range === 'month') flowPeriodLabel.textContent = `• ${spanText}`;
            else if (range === 'week') flowPeriodLabel.textContent = `• Last 7 Days`;
            else flowPeriodLabel.textContent = `• Today (${new Date().toLocaleDateString('en-US', { day: 'numeric', month: 'short' })})`;
        }

        const qBtn = document.querySelector('.stats-tab-btn[data-range="quarter"]');
        if (qBtn) {
            qBtn.textContent = '3 Months';
        }

        // Active Work Window (Today's Hours OR Period Date Span)
        const workWinLabel = document.getElementById('stats-kpi-work-window-label');
        const workWinElem = document.getElementById('stats-kpi-work-window');
        const workWinSubElem = workWinElem ? workWinElem.nextElementSibling : null;
        if (workWinElem) {
            if (range === 'today') {
                if (workWinLabel) workWinLabel.textContent = 'Work Window';
                const todayRange = userData.workTimeRangesByDate && userData.workTimeRangesByDate[todayDateStr];
                if (todayRange && todayRange.firstStart) {
                    workWinElem.innerHTML = `<span>${todayRange.firstStart}</span><span style="opacity:0.6;margin:0 1px;">—</span><span>${todayRange.lastActive}</span>`;
                } else {
                    workWinElem.textContent = 'Not started';
                }
                if (workWinSubElem) workWinSubElem.textContent = 'Active hours';
            } else {
                if (workWinLabel) workWinLabel.textContent = 'Timeframe';
                if (range === 'quarter') {
                    const parts = spanText.split(' ');
                    if (parts.length >= 3) {
                        workWinElem.innerHTML = `<span>${parts.slice(0, -1).join(' ')}</span> <span>${parts[parts.length - 1]}</span>`;
                    } else {
                        workWinElem.textContent = spanText;
                    }
                } else {
                    workWinElem.textContent = spanText;
                }
                if (workWinSubElem) {
                    workWinSubElem.textContent = range === 'quarter' ? '3 months span' : (range === 'month' ? '30 days span' : '7 days span');
                }
            }
        }

        // Productivity Score Calculation
        const sortedApps = Object.keys(appMap)
            .map(exe => ({ exe, secs: appMap[exe] }))
            .sort((a, b) => b.secs - a.secs);

        let productiveSecs = 0;
        let neutralSecs = 0;
        let distractingSecs = 0;

        sortedApps.forEach(item => {
            const cat = getAppCategory(item.exe);
            if (cat === 'productive') productiveSecs += item.secs;
            else if (cat === 'neutral') neutralSecs += item.secs;
            else distractingSecs += item.secs;
        });

        const prodScore = totalSecs > 0 ? Math.min(100, Math.round(((productiveSecs * 1.0 + neutralSecs * 0.5) / totalSecs) * 100)) : 100;
        const prodScoreElem = document.getElementById('stats-kpi-prod-score');
        if (prodScoreElem) {
            prodScoreElem.textContent = `${prodScore}%`;
            prodScoreElem.style.color = prodScore >= 80 ? 'var(--accent-green)' : (prodScore >= 50 ? 'var(--accent-blue)' : '#f87171');
        }

        const ignoredCount = (userData.ignoredNudgeStatsByDate && userData.ignoredNudgeStatsByDate[todayDateStr]) || 0;
        const distractionsElem = document.getElementById('stats-kpi-distractions');
        if (distractionsElem) {
            distractionsElem.textContent = `${ignoredCount}x`;
            distractionsElem.style.color = ignoredCount === 0 ? 'var(--accent-green)' : (ignoredCount <= 2 ? '#fb923c' : '#f87171');
        }

        // Goal Bar
        document.getElementById('stats-goal-target-text').textContent = `${(targetGoalMins / 60).toFixed(1)} hrs (${range})`;
        const remMins = Math.max(0, targetGoalMins - totalMins);
        document.getElementById('stats-goal-status-text').textContent = remMins > 0 ? `${(remMins / 60).toFixed(1)} hrs remaining` : `Goal Met! 🎉`;
        document.getElementById('stats-goal-bar-fill').style.width = `${goalPercent}%`;

        // Ranked App List
        const statsAppListContainer = document.getElementById('stats-app-list-container');
        statsAppListContainer.innerHTML = '';

        renderWeeklyChart();
        renderHeatmapMatrix();
        renderPeakHoursChart(range);

        if (sortedApps.length === 0) {
            statsAppListContainer.innerHTML = `<div style="text-align:center; padding:24px; color:var(--text-muted); font-size:12px;">No focus data recorded for this period yet. Start a focus session to track app statistics!</div>`;
            return;
        }

        const maxAppSecs = sortedApps[0].secs || 1;
        const blockedList = (userData.blockedApps || []).map(a => a.toLowerCase());

        sortedApps.forEach(item => {
            const percent = totalSecs > 0 ? Math.round((item.secs / totalSecs) * 100) : 0;
            const relativePercent = Math.round((item.secs / maxAppSecs) * 100);
            const lowerExe = item.exe.toLowerCase();
            const category = getAppCategory(item.exe);
            const isBlocked = blockedList.includes(lowerExe);

            // Lookup real extracted icon from userData.appIconMap or cachedAppList
            let appIcon = (userData.appIconMap && userData.appIconMap[lowerExe]) || '';
            if (!appIcon && cachedAppList && cachedAppList.length > 0) {
                const found = cachedAppList.find(a => a.exeName && a.exeName.toLowerCase() === lowerExe);
                if (found && found.icon && found.icon.length > 30) appIcon = found.icon;
            }
            if (!appIcon) appIcon = defaultIconSvg;

            const div = document.createElement('div');
            div.className = 'stats-app-item';
            div.setAttribute('data-exe', lowerExe);

            const topDiv = document.createElement('div');
            topDiv.className = 'stats-app-top';

            const nameDiv = document.createElement('div');
            nameDiv.className = 'stats-app-name';

            const imgNode = document.createElement('img');
            imgNode.className = 'stats-app-icon';
            imgNode.style.cssText = 'width: 16px; height: 16px; object-fit: contain; vertical-align: -3px; margin-right: 8px; border-radius: 3px;';
            imgNode.src = appIcon || defaultIconSvg;
            imgNode.onerror = () => { imgNode.src = defaultIconSvg; };

            const spanNode = document.createElement('span');
            spanNode.textContent = item.exe;

            nameDiv.appendChild(imgNode);
            nameDiv.appendChild(spanNode);

            // Category Pill (Clickable)
            const catPill = document.createElement('span');
            catPill.className = `cat-pill ${category}`;
            catPill.innerHTML = `<svg width="7" height="7" viewBox="0 0 24 24" fill="currentColor" style="flex-shrink:0;"><circle cx="12" cy="12" r="10"/></svg> <span>${category}</span>`;
            catPill.title = 'Click to toggle category (Productive / Neutral / Distracting)';
            catPill.addEventListener('click', (e) => {
                e.stopPropagation();
                toggleAppCategory(item.exe);
            });
            nameDiv.appendChild(catPill);

            // Live Overstay Streak Badge (Compact, real-time seconds & minutes)
            if (lastContinuousExe && lastContinuousExe.toLowerCase() === lowerExe && continuousNeutralSec > 0 && category !== 'productive') {
                const limitMins = parseInt(userData.neutralOverstayIntervalMins) || 30;
                const liveBadge = document.createElement('span');
                liveBadge.id = 'live-overstay-streak-badge';
                liveBadge.className = 'live-streak-pill';
                liveBadge.style.cssText = 'background: rgba(96, 205, 255, 0.12); border: 1px solid rgba(96, 205, 255, 0.3); color: #60cdff; font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 4px; display: inline-flex; align-items: center; margin-left: 6px;';
                const liveMins = Math.floor(continuousNeutralSec / 60);
                if (liveMins >= limitMins) {
                    liveBadge.style.background = 'rgba(248, 113, 113, 0.15)';
                    liveBadge.style.borderColor = 'rgba(248, 113, 113, 0.35)';
                    liveBadge.style.color = '#f87171';
                } else if (liveMins >= limitMins * 0.7) {
                    liveBadge.style.background = 'rgba(251, 146, 60, 0.15)';
                    liveBadge.style.borderColor = 'rgba(251, 146, 60, 0.35)';
                    liveBadge.style.color = '#fb923c';
                }
                liveBadge.textContent = formatLiveStreak(continuousNeutralSec, limitMins);
                liveBadge.title = `Active continuous duration: ${continuousNeutralSec}s (Limit: ${limitMins}m)`;
                nameDiv.appendChild(liveBadge);
            }

            // Right side: Time and Quick Block Button
            const actionsDiv = document.createElement('div');
            actionsDiv.className = 'stats-app-actions';

            const timeDiv = document.createElement('div');
            timeDiv.className = 'stats-app-time';
            timeDiv.textContent = `${formatSecs(item.secs)} (${percent}%)`;

            const blockBtn = document.createElement('button');
            blockBtn.className = `btn-quick-block ${isBlocked ? 'blocked' : ''}`;
            blockBtn.innerHTML = isBlocked
                ? '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect><path d="M7 11V7a5 5 0 0 1 10 0v4"></path></svg> <span>Blocked</span>'
                : '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"></line></svg> <span>Block</span>';
            blockBtn.title = isBlocked ? 'App is restricted during focus. Click to unblock.' : 'Click to restrict this app during focus sessions.';
            blockBtn.addEventListener('click', (e) => {
                e.stopPropagation();
                toggleQuickBlockApp(item.exe);
            });

            actionsDiv.appendChild(timeDiv);
            actionsDiv.appendChild(blockBtn);

            topDiv.appendChild(nameDiv);
            topDiv.appendChild(actionsDiv);

            const barBgDiv = document.createElement('div');
            barBgDiv.className = 'stats-app-bar-bg';

            const barFillDiv = document.createElement('div');
            barFillDiv.className = 'stats-app-bar-fill';
            barFillDiv.style.width = `${relativePercent}%`;

            barBgDiv.appendChild(barFillDiv);

            div.appendChild(topDiv);
            div.appendChild(barBgDiv);

            statsAppListContainer.appendChild(div);
        });
    }

    // ============================================================
    // Share / Report Card Modal Logic & Canvas Image Exporter
    // ============================================================
    const btnOpenShareCard = document.getElementById('btn-open-share-card');
    const shareCardModal = document.getElementById('share-card-modal');
    const btnCloseShareModal = document.getElementById('btn-close-share-modal');
    const btnCopyShareText = document.getElementById('btn-copy-share-text');
    const btnDownloadShareCard = document.getElementById('btn-download-share-card');

    function openShareCardModal() {
        if (!shareCardModal) return;
        shareCardModal.classList.add('active');

        const todayStats = (userData.appStatsByDate && userData.appStatsByDate[todayDateStr]) || {};
        let totalSecs = 0;
        const appMap = {};
        Object.keys(todayStats).forEach(exe => {
            const sec = todayStats[exe] || 0;
            totalSecs += sec;
            appMap[exe] = sec;
        });

        const sortedApps = Object.keys(appMap)
            .map(exe => ({ exe, secs: appMap[exe] }))
            .sort((a, b) => b.secs - a.secs);

        let productiveSecs = 0;
        let neutralSecs = 0;
        sortedApps.forEach(item => {
            const cat = getAppCategory(item.exe);
            if (cat === 'productive') productiveSecs += item.secs;
            else if (cat === 'neutral') neutralSecs += item.secs;
        });

        const prodScore = totalSecs > 0 ? Math.min(100, Math.round(((productiveSecs * 1.0 + neutralSecs * 0.5) / totalSecs) * 100)) : 100;
        const totalMins = Math.floor(totalSecs / 60);
        const targetGoalMins = (userData.dailyGoalHours || 1) * 60;
        const goalPercent = Math.min(100, Math.round((totalMins / targetGoalMins) * 100));

        // Format Date
        const now = new Date();
        const dateOptions = { weekday: 'short', day: 'numeric', month: 'short', year: 'numeric' };
        document.getElementById('share-card-date-badge').textContent = now.toLocaleDateString('en-US', dateOptions);

        // Fill Hero
        document.getElementById('share-hero-time').textContent = formatSecs(totalSecs);
        document.getElementById('share-card-score-badge').textContent = `${prodScore}% Focus Score`;
        document.getElementById('share-card-streak-badge').textContent = `${userData.streakDays || 0} Days Streak`;

        // Fill Progress
        const todayRange = (userData.workTimeRangesByDate && userData.workTimeRangesByDate[todayDateStr]) || {};
        const winStr = todayRange.firstStart ? `${todayRange.firstStart} - ${todayRange.lastActive}` : '09:00 AM - Current';
        document.getElementById('share-card-goal-label').textContent = `Goal Met: ${goalPercent}% (${(userData.dailyGoalHours || 1).toFixed(1)} hrs)`;
        document.getElementById('share-card-window-label').textContent = `Window: ${winStr}`;
        document.getElementById('share-card-bar-fill').style.width = `${goalPercent}%`;

        // Fill Top 3 Apps
        const appsListContainer = document.getElementById('share-card-apps-list');
        appsListContainer.innerHTML = '';

        const top3 = sortedApps.slice(0, 3);
        if (top3.length === 0) {
            appsListContainer.innerHTML = `<div style="text-align:center; font-size:10px; color:var(--text-muted); padding:6px;">No focus apps logged today yet.</div>`;
        } else {
            top3.forEach(item => {
                const lowerExe = item.exe.toLowerCase();
                let appIcon = (userData.appIconMap && userData.appIconMap[lowerExe]) || '';
                if (!appIcon && cachedAppList && cachedAppList.length > 0) {
                    const found = cachedAppList.find(a => a.exeName && a.exeName.toLowerCase() === lowerExe);
                    if (found && found.icon && found.icon.length > 30) appIcon = found.icon;
                }
                if (!appIcon) appIcon = defaultIconSvg;

                const percent = totalSecs > 0 ? Math.round((item.secs / totalSecs) * 100) : 0;
                const row = document.createElement('div');
                row.className = 'share-app-row';

                const rowLeft = document.createElement('div');
                rowLeft.className = 'share-app-row-left';

                const iconImg = document.createElement('img');
                iconImg.className = 'share-app-row-icon';
                iconImg.src = appIcon;
                iconImg.onerror = () => { iconImg.src = defaultIconSvg; };

                const nameSpan = document.createElement('span');
                nameSpan.className = 'share-app-row-name';
                nameSpan.textContent = item.exe;

                rowLeft.appendChild(iconImg);
                rowLeft.appendChild(nameSpan);

                const timeSpan = document.createElement('span');
                timeSpan.className = 'share-app-row-time';
                timeSpan.textContent = `${formatSecs(item.secs)} (${percent}%)`;

                row.appendChild(rowLeft);
                row.appendChild(timeSpan);
                appsListContainer.appendChild(row);
            });
        }
    }

    btnOpenShareCard?.addEventListener('click', openShareCardModal);
    btnCloseShareModal?.addEventListener('click', () => {
        if (shareCardModal) shareCardModal.classList.remove('active');
    });

    // Copy formatted report text
    btnCopyShareText?.addEventListener('click', () => {
        const todayStats = (userData.appStatsByDate && userData.appStatsByDate[todayDateStr]) || {};
        let totalSecs = 0;
        const appMap = {};
        Object.keys(todayStats).forEach(exe => {
            const sec = todayStats[exe] || 0;
            totalSecs += sec;
            appMap[exe] = sec;
        });

        const sortedApps = Object.keys(appMap)
            .map(exe => ({ exe, secs: appMap[exe] }))
            .sort((a, b) => b.secs - a.secs);

        let productiveSecs = 0;
        let neutralSecs = 0;
        sortedApps.forEach(item => {
            const cat = getAppCategory(item.exe);
            if (cat === 'productive') productiveSecs += item.secs;
            else if (cat === 'neutral') neutralSecs += item.secs;
        });
        const prodScore = totalSecs > 0 ? Math.min(100, Math.round(((productiveSecs * 1.0 + neutralSecs * 0.5) / totalSecs) * 100)) : 100;

        const now = new Date();
        const dateStr = now.toLocaleDateString('en-US', { weekday: 'short', day: 'numeric', month: 'short', year: 'numeric' });
        const todayRange = (userData.workTimeRangesByDate && userData.workTimeRangesByDate[todayDateStr]) || {};
        const winStr = todayRange.firstStart ? `${todayRange.firstStart} - ${todayRange.lastActive}` : 'Active session';

        let appText = '';
        sortedApps.slice(0, 3).forEach((item, idx) => {
            const percent = totalSecs > 0 ? Math.round((item.secs / totalSecs) * 100) : 0;
            appText += `\n  ${idx + 1}. ${item.exe} — ${formatSecs(item.secs)} (${percent}%)`;
        });

        const textReport = `🌱 FocusGrow Productivity Report — ${dateStr}
⏱️ Total Focus: ${formatSecs(totalSecs)}
⚡ Focus Efficiency: ${prodScore}%
🔥 Streak: ${userData.streakDays || 0} Days
🕒 Active Window: ${winStr}
🏆 Top Applications:${appText || ' None yet'}

#StayFocused #FocusGrow`;

        navigator.clipboard.writeText(textReport).then(() => {
            sendNotification('Report Copied', 'Daily focus report copied to clipboard!', 'focus');
        }).catch(() => {
            sendNotification('Copy Failed', 'Unable to access clipboard.', 'focus');
        });
    });

    // High-Resolution Canvas Card Exporter
    btnDownloadShareCard?.addEventListener('click', () => {
        const todayStats = (userData.appStatsByDate && userData.appStatsByDate[todayDateStr]) || {};
        let totalSecs = 0;
        const appMap = {};
        Object.keys(todayStats).forEach(exe => {
            const sec = todayStats[exe] || 0;
            totalSecs += sec;
            appMap[exe] = sec;
        });

        const sortedApps = Object.keys(appMap)
            .map(exe => ({ exe, secs: appMap[exe] }))
            .sort((a, b) => b.secs - a.secs);

        let productiveSecs = 0;
        let neutralSecs = 0;
        sortedApps.forEach(item => {
            const cat = getAppCategory(item.exe);
            if (cat === 'productive') productiveSecs += item.secs;
            else if (cat === 'neutral') neutralSecs += item.secs;
        });
        const prodScore = totalSecs > 0 ? Math.min(100, Math.round(((productiveSecs * 1.0 + neutralSecs * 0.5) / totalSecs) * 100)) : 100;

        const canvas = document.createElement('canvas');
        const width = 800;
        const height = 520;
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext('2d');

        // Background gradient
        const bgGrad = ctx.createLinearGradient(0, 0, width, height);
        bgGrad.addColorStop(0, '#121824');
        bgGrad.addColorStop(1, '#080c10');
        ctx.fillStyle = bgGrad;
        ctx.fillRect(0, 0, width, height);

        // Neon ambient glow circles
        const glow1 = ctx.createRadialGradient(width - 50, 50, 10, width - 50, 50, 220);
        glow1.addColorStop(0, 'rgba(96, 205, 255, 0.28)');
        glow1.addColorStop(1, 'rgba(96, 205, 255, 0)');
        ctx.fillStyle = glow1;
        ctx.fillRect(0, 0, width, height);

        const glow2 = ctx.createRadialGradient(50, height - 50, 10, 50, height - 50, 220);
        glow2.addColorStop(0, 'rgba(74, 222, 128, 0.24)');
        glow2.addColorStop(1, 'rgba(74, 222, 128, 0)');
        ctx.fillStyle = glow2;
        ctx.fillRect(0, 0, width, height);

        // Card Border
        ctx.strokeStyle = 'rgba(96, 205, 255, 0.4)';
        ctx.lineWidth = 3;
        ctx.strokeRect(10, 10, width - 20, height - 20);

        // Header: Brand & Date
        ctx.fillStyle = '#60cdff';
        ctx.font = 'bold 28px "Segoe UI", sans-serif';
        ctx.fillText('🌱 FocusGrow', 40, 60);

        ctx.fillStyle = 'rgba(255, 255, 255, 0.5)';
        ctx.font = '14px "Segoe UI", sans-serif';
        ctx.fillText('Daily Focus Intelligence', 40, 82);

        const now = new Date();
        const dateStr = now.toLocaleDateString('en-US', { weekday: 'short', day: 'numeric', month: 'short', year: 'numeric' });
        ctx.fillStyle = 'rgba(255, 255, 255, 0.8)';
        ctx.font = 'bold 16px "Segoe UI", sans-serif';
        ctx.textAlign = 'right';
        ctx.fillText(dateStr, width - 40, 60);
        ctx.textAlign = 'left';

        // Horizontal Separator
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(40, 100);
        ctx.lineTo(width - 40, 100);
        ctx.stroke();

        // Hero Metric: Big Time
        ctx.fillStyle = '#8899a6';
        ctx.font = 'bold 14px "Segoe UI", sans-serif';
        ctx.fillText('TOTAL FOCUS TIME', 40, 140);

        ctx.fillStyle = '#ffffff';
        ctx.font = 'bold 56px "Segoe UI", sans-serif';
        ctx.fillText(formatSecs(totalSecs), 40, 200);

        // Badges on Right
        // Efficiency Badge
        ctx.fillStyle = 'rgba(74, 222, 128, 0.15)';
        ctx.fillRect(width - 260, 125, 220, 36);
        ctx.strokeStyle = 'rgba(74, 222, 128, 0.4)';
        ctx.strokeRect(width - 260, 125, 220, 36);
        ctx.fillStyle = '#4ade80';
        ctx.font = 'bold 15px "Segoe UI", sans-serif';
        ctx.fillText(`⚡ ${prodScore}% Efficiency Score`, width - 245, 149);

        // Streak Badge
        ctx.fillStyle = 'rgba(251, 146, 60, 0.15)';
        ctx.fillRect(width - 260, 170, 220, 36);
        ctx.strokeStyle = 'rgba(251, 146, 60, 0.4)';
        ctx.strokeRect(width - 260, 170, 220, 36);
        ctx.fillStyle = '#fb923c';
        ctx.font = 'bold 15px "Segoe UI", sans-serif';
        ctx.fillText(`🔥 ${userData.streakDays || 0} Days Streak`, width - 245, 194);

        // Top Apps Section
        ctx.fillStyle = '#8899a6';
        ctx.font = 'bold 13px "Segoe UI", sans-serif';
        ctx.fillText('TOP FOCUS APPLICATIONS', 40, 250);

        const top3 = sortedApps.slice(0, 3);
        let startY = 270;
        if (top3.length === 0) {
            ctx.fillStyle = 'rgba(255, 255, 255, 0.4)';
            ctx.font = '14px "Segoe UI", sans-serif';
            ctx.fillText('No focus activity recorded today.', 40, startY + 20);
        } else {
            top3.forEach((item, idx) => {
                ctx.fillStyle = 'rgba(255, 255, 255, 0.04)';
                ctx.fillRect(40, startY, width - 80, 40);
                ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
                ctx.strokeRect(40, startY, width - 80, 40);

                ctx.fillStyle = '#ffffff';
                ctx.font = 'bold 14px "Segoe UI", sans-serif';
                ctx.fillText(`${idx + 1}.  ${item.exe}`, 60, startY + 25);

                const percent = totalSecs > 0 ? Math.round((item.secs / totalSecs) * 100) : 0;
                ctx.fillStyle = '#60cdff';
                ctx.textAlign = 'right';
                ctx.fillText(`${formatSecs(item.secs)} (${percent}%)`, width - 60, startY + 25);
                ctx.textAlign = 'left';

                startY += 48;
            });
        }

        // Footer
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
        ctx.beginPath();
        ctx.moveTo(40, height - 55);
        ctx.lineTo(width - 40, height - 55);
        ctx.stroke();

        ctx.fillStyle = 'rgba(255, 255, 255, 0.5)';
        ctx.font = 'italic 13px "Segoe UI", sans-serif';
        ctx.fillText('"Deep focus creates masterpiece results."', 40, height - 30);

        ctx.textAlign = 'right';
        ctx.fillStyle = 'rgba(96, 205, 255, 0.7)';
        ctx.font = 'bold 12px "Segoe UI", sans-serif';
        ctx.fillText('FocusGrow OS • Windows', width - 40, height - 30);

        // Trigger Download
        const link = document.createElement('a');
        link.download = `FocusGrow-Productivity-Card-${todayDateStr}.png`;
        link.href = canvas.toDataURL('image/png');
        link.click();

        sendNotification('Card Downloaded', 'Focus productivity card exported as PNG image!', 'focus');
    });

    // ============================================================
    // Futuristic 1-3 Month Flow & Pattern Intelligence Modal
    // ============================================================
    const patternReportModal = document.getElementById('pattern-report-modal');
    const btnClosePatternModal = document.getElementById('btn-close-pattern-modal');
    const btnOpenPatternReport = document.getElementById('btn-open-pattern-report');
    const btnCopyPatternText = document.getElementById('btn-copy-pattern-text');
    const btnDownloadPatternCard = document.getElementById('btn-download-pattern-card');
    let currentPatternPeriod = 30;

    function renderPatternReportContent(days = 30) {
        currentPatternPeriod = days;

        // Update pill buttons active state
        document.querySelectorAll('.pattern-pill-btn').forEach(btn => {
            btn.classList.toggle('active', parseInt(btn.getAttribute('data-period'), 10) === days);
        });

        // Gather target dates
        const targetDates = [];
        const now = new Date();
        for (let i = 0; i < days; i++) {
            const d = new Date(now);
            d.setDate(d.getDate() - i);
            targetDates.push(getLocalDateStr(d));
        }

        let totalSecs = 0;
        let activeDays = 0;
        const appMap = {};
        const hourlyBuckets = new Array(24).fill(0);
        userData.appStatsByDate = userData.appStatsByDate || {};

        targetDates.forEach(dateStr => {
            let dayHasFocus = false;

            // App breakdown
            const dayStats = userData.appStatsByDate[dateStr];
            if (dayStats) {
                Object.keys(dayStats).forEach(exe => {
                    const sec = dayStats[exe] || 0;
                    if (sec > 0) {
                        totalSecs += sec;
                        appMap[exe] = (appMap[exe] || 0) + sec;
                        dayHasFocus = true;
                    }
                });
            }

            // Hourly focus buckets
            if (userData.hourlyFocusStats && userData.hourlyFocusStats[dateStr]) {
                const dayHours = userData.hourlyFocusStats[dateStr];
                for (let h = 0; h < 24; h++) {
                    if (dayHours[h]) {
                        hourlyBuckets[h] += dayHours[h];
                        if (dayHours[h] > 0) dayHasFocus = true;
                    }
                }
            } else {
                let dayMins = (dateStr === todayDateStr) ? (userData.completedMinutesToday || 0) : ((userData.completedMinutesByDate && userData.completedMinutesByDate[dateStr]) || 0);
                if (dayMins > 0) {
                    dayHasFocus = true;
                    const rangeInfo = (userData.workTimeRangesByDate && userData.workTimeRangesByDate[dateStr]);
                    let startH = 9, endH = 18;
                    if (rangeInfo && rangeInfo.firstStart && rangeInfo.lastActive) {
                        const parseH = (tStr) => {
                            const match = tStr.match(/(\d+):(\d+)\s*(AM|PM)?/i);
                            if (match) {
                                let h = parseInt(match[1], 10);
                                const isPM = (match[3] || '').toUpperCase() === 'PM';
                                const isAM = (match[3] || '').toUpperCase() === 'AM';
                                if (isPM && h < 12) h += 12;
                                if (isAM && h === 12) h = 0;
                                return h;
                            }
                            return 9;
                        };
                        startH = parseH(rangeInfo.firstStart);
                        endH = parseH(rangeInfo.lastActive);
                        if (endH < startH) endH = Math.min(23, startH + 4);
                    }
                    const numHours = Math.max(1, (endH - startH + 1));
                    const baseMins = Math.floor(dayMins / numHours);
                    let rem = dayMins % numHours;
                    for (let h = startH; h <= endH; h++) {
                        const bonus = rem > 0 ? 1 : 0;
                        if (rem > 0) rem--;
                        hourlyBuckets[h] += (baseMins + bonus);
                    }
                }
            }

            if (dayHasFocus) activeDays++;
        });

        // Productivity efficiency score
        const sortedApps = Object.keys(appMap)
            .map(exe => ({ exe, secs: appMap[exe] }))
            .sort((a, b) => b.secs - a.secs);

        let productiveSecs = 0;
        let neutralSecs = 0;
        sortedApps.forEach(item => {
            const cat = getAppCategory(item.exe);
            if (cat === 'productive') productiveSecs += item.secs;
            else if (cat === 'neutral') neutralSecs += item.secs;
        });
        const prodScore = totalSecs > 0 ? Math.min(100, Math.round(((productiveSecs * 1.0 + neutralSecs * 0.5) / totalSecs) * 100)) : 100;

        // Daily average
        const avgSecsPerActiveDay = activeDays > 0 ? Math.round(totalSecs / activeDays) : 0;

        // Peak 2-Hour Window calculation
        let peakWindowStart = 0;
        let peakWindowSum = 0;
        let highestSingleHour = 0;
        let highestSingleVal = 0;

        for (let h = 0; h < 24; h++) {
            const twoHourSum = hourlyBuckets[h] + hourlyBuckets[(h + 1) % 24];
            if (twoHourSum > peakWindowSum) {
                peakWindowSum = twoHourSum;
                peakWindowStart = h;
            }
            if (hourlyBuckets[h] > highestSingleVal) {
                highestSingleVal = hourlyBuckets[h];
                highestSingleHour = h;
            }
        }

        const peakHour = peakWindowStart;
        const peakWindow = `${String(peakHour).padStart(2, '0')}:00 — ${String((peakHour + 2) % 24).padStart(2, '0')}:00`;

        // Chronotype Persona evaluation
        const totalAggMins = hourlyBuckets.reduce((a, b) => a + b, 0);
        const todayRangeInfo = (userData.workTimeRangesByDate && userData.workTimeRangesByDate[todayDateStr]);
        const chronoInfo = evaluateChronotype(hourlyBuckets, peakHour, totalAggMins, todayRangeInfo);

        // Fill KPI Cards
        const spanText = getPeriodMonthSpanText(days);
        const shortSpan = spanText.replace(' ' + new Date().getFullYear(), '');

        const dateRangeElem = document.getElementById('pattern-card-date-range');
        if (dateRangeElem) {
            dateRangeElem.textContent = `Work Rhythm & Chronotype Intelligence • ${spanText} (${days === 90 ? 'Last 3 Months' : 'Last 1 Month'})`;
        }

        const pill30 = document.getElementById('pill-period-30');
        const pill90 = document.getElementById('pill-period-90');
        if (pill30) pill30.textContent = `1 Month (${getPeriodMonthSpanText(30).replace(' ' + new Date().getFullYear(), '')})`;
        if (pill90) pill90.textContent = `3 Months (${getPeriodMonthSpanText(90).replace(' ' + new Date().getFullYear(), '')})`;

        const totalTimeElem = document.getElementById('pattern-stat-total-time');
        const activeDaysElem = document.getElementById('pattern-stat-active-days');
        const dailyAvgElem = document.getElementById('pattern-stat-daily-avg');
        const efficiencyElem = document.getElementById('pattern-stat-efficiency');
        const peakStreakElem = document.getElementById('pattern-stat-peak-streak');

        if (totalTimeElem) totalTimeElem.textContent = formatSecs(totalSecs);
        if (activeDaysElem) activeDaysElem.textContent = `${activeDays} / ${days} active days (${shortSpan})`;
        if (dailyAvgElem) dailyAvgElem.textContent = formatSecs(avgSecsPerActiveDay);
        if (efficiencyElem) efficiencyElem.textContent = `${prodScore}%`;
        if (peakStreakElem) peakStreakElem.textContent = `Streak: ${userData.streakDays || 0} Days`;

        // Fill Persona Card
        const personaBadge = document.getElementById('pattern-persona-badge');
        const personaDesc = document.getElementById('pattern-persona-desc');
        const peakTag = document.getElementById('pattern-peak-tag');
        const wavePeakLabel = document.getElementById('pattern-wave-peak-label');

        if (personaBadge) {
            personaBadge.textContent = chronoInfo.persona;
            personaBadge.style.color = chronoInfo.color;
        }
        if (personaDesc) personaDesc.textContent = chronoInfo.desc;
        if (peakTag) peakTag.textContent = `Peak: ${peakWindow}`;
        if (wavePeakLabel) wavePeakLabel.textContent = `Peak Window: ${peakWindow}`;

        // Render 24-Hour Wave SVG
        const areaPath = document.getElementById('pattern-wave-area');
        const linePath = document.getElementById('pattern-wave-line');
        const patternMarker = document.getElementById('pattern-peak-marker');
        const patternDot = patternMarker ? patternMarker.querySelector('.peak-marker-dot') : null;
        const patternPulse = patternMarker ? patternMarker.querySelector('.peak-marker-pulse') : null;

        if (areaPath && linePath) {
            const maxH = Math.max(1, ...hourlyBuckets);
            const points = [];
            for (let h = 0; h < 24; h++) {
                const x = (h / 23) * 500;
                const val = hourlyBuckets[h];
                const y = (maxH > 0 && val > 0) ? (76 - ((val / maxH) * 58)) : 76;
                points.push({ x, y, val, hour: h });
            }

            let lineD = `M ${points[0].x.toFixed(1)} ${points[0].y.toFixed(1)}`;
            for (let i = 0; i < points.length - 1; i++) {
                const p0 = points[i === 0 ? 0 : i - 1];
                const p1 = points[i];
                const p2 = points[i + 1];
                const p3 = points[i + 2 >= points.length ? points.length - 1 : i + 2];

                const cp1x = p1.x + (p2.x - p0.x) / 5;
                const cp1y = p1.y + (p2.y - p0.y) / 5;
                const cp2x = p2.x - (p3.x - p1.x) / 5;
                const cp2y = p2.y - (p3.y - p1.y) / 5;

                lineD += ` C ${cp1x.toFixed(1)} ${cp1y.toFixed(1)}, ${cp2x.toFixed(1)} ${cp2y.toFixed(1)}, ${p2.x.toFixed(1)} ${p2.y.toFixed(1)}`;
            }

            const areaD = lineD + ` L 500 76 L 0 76 Z`;
            linePath.setAttribute('d', lineD);
            areaPath.setAttribute('d', areaD);

            // 100% Mathematically Perfect Circle (Never Gepeng)
            if (patternMarker && highestSingleVal > 0) {
                const peakPt = points[highestSingleHour];
                patternMarker.style.left = `${(peakPt.x / 500) * 100}%`;
                patternMarker.style.top = `${(peakPt.y / 90) * 100}%`;
                patternMarker.style.display = 'flex';
                if (patternDot) {
                    patternDot.style.background = chronoInfo.color || '#60cdff';
                    patternDot.style.boxShadow = `0 0 10px ${chronoInfo.color || '#60cdff'}, 0 0 4px #ffffff`;
                }
                if (patternPulse) {
                    patternPulse.style.background = (chronoInfo.color || '#60cdff') + '44';
                }
            } else if (patternMarker) {
                patternMarker.style.display = 'none';
            }
        }

        // Render Day-Part Time Distribution
        const morningMins = hourlyBuckets.slice(5, 12).reduce((a, b) => a + b, 0);
        const afternoonMins = hourlyBuckets.slice(12, 18).reduce((a, b) => a + b, 0);
        const nightMins = hourlyBuckets.slice(18, 24).reduce((a, b) => a + b, 0);
        const midnightMins = hourlyBuckets.slice(0, 5).reduce((a, b) => a + b, 0);
        const sumParts = Math.max(1, morningMins + afternoonMins + nightMins + midnightMins);

        const morningPct = Math.round((morningMins / sumParts) * 100);
        const afternoonPct = Math.round((afternoonMins / sumParts) * 100);
        const nightPct = Math.round((nightMins / sumParts) * 100);
        const midnightPct = Math.max(0, 100 - (morningPct + afternoonPct + nightPct));

        const distMorning = document.getElementById('pattern-dist-morning');
        const pctMorning = document.getElementById('pattern-pct-morning');
        const distAfternoon = document.getElementById('pattern-dist-afternoon');
        const pctAfternoon = document.getElementById('pattern-pct-afternoon');
        const distNight = document.getElementById('pattern-dist-night');
        const pctNight = document.getElementById('pattern-pct-night');
        const distMidnight = document.getElementById('pattern-dist-midnight');
        const pctMidnight = document.getElementById('pattern-pct-midnight');

        if (distMorning && pctMorning) { distMorning.style.width = `${morningPct}%`; pctMorning.textContent = `${morningPct}%`; }
        if (distAfternoon && pctAfternoon) { distAfternoon.style.width = `${afternoonPct}%`; pctAfternoon.textContent = `${afternoonPct}%`; }
        if (distNight && pctNight) { distNight.style.width = `${nightPct}%`; pctNight.textContent = `${nightPct}%`; }
        if (distMidnight && pctMidnight) { distMidnight.style.width = `${midnightPct}%`; pctMidnight.textContent = `${midnightPct}%`; }

        // Render Top Software List
        const appsListContainer = document.getElementById('pattern-card-apps-list');
        if (appsListContainer) {
            appsListContainer.innerHTML = '';
            const top3 = sortedApps.slice(0, 3);
            if (top3.length === 0) {
                appsListContainer.innerHTML = `<div style="font-size:10px; color:var(--text-muted); padding:6px 0;">No application focus records logged in this period.</div>`;
            } else {
                top3.forEach(item => {
                    const lowerExe = item.exe.toLowerCase();
                    let appIcon = (userData.appIconMap && userData.appIconMap[lowerExe]) || '';
                    if (!appIcon && cachedAppList && cachedAppList.length > 0) {
                        const found = cachedAppList.find(a => a.exeName && a.exeName.toLowerCase() === lowerExe);
                        if (found && found.icon && found.icon.length > 30) appIcon = found.icon;
                    }
                    if (!appIcon) appIcon = defaultIconSvg;

                    const percent = totalSecs > 0 ? Math.round((item.secs / totalSecs) * 100) : 0;
                    const row = document.createElement('div');
                    row.className = 'share-app-row';
                    row.style.padding = '4px 6px';

                    const rowLeft = document.createElement('div');
                    rowLeft.className = 'share-app-row-left';

                    const iconImg = document.createElement('img');
                    iconImg.className = 'share-app-row-icon';
                    iconImg.src = appIcon;
                    iconImg.onerror = () => { iconImg.src = defaultIconSvg; };

                    const nameSpan = document.createElement('span');
                    nameSpan.className = 'share-app-row-name';
                    nameSpan.style.fontSize = '11px';
                    nameSpan.textContent = item.exe;

                    rowLeft.appendChild(iconImg);
                    rowLeft.appendChild(nameSpan);

                    const timeSpan = document.createElement('span');
                    timeSpan.className = 'share-app-row-time';
                    timeSpan.style.fontSize = '11px';
                    timeSpan.textContent = `${formatSecs(item.secs)} (${percent}%)`;

                    row.appendChild(rowLeft);
                    row.appendChild(timeSpan);
                    appsListContainer.appendChild(row);
                });
            }
        }
    }

    function openPatternReportModal(days = 30) {
        if (!patternReportModal) return;
        patternReportModal.classList.add('active');
        renderPatternReportContent(days);
    }

    btnOpenPatternReport?.addEventListener('click', (e) => {
        e.stopPropagation();
        openPatternReportModal(30);
    });

    btnClosePatternModal?.addEventListener('click', () => {
        if (patternReportModal) patternReportModal.classList.remove('active');
    });

    document.querySelectorAll('.pattern-pill-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            const days = parseInt(btn.getAttribute('data-period'), 10) || 30;
            renderPatternReportContent(days);
        });
    });

    // Copy Pattern Report Summary
    btnCopyPatternText?.addEventListener('click', () => {
        const days = currentPatternPeriod;
        const targetDates = [];
        const now = new Date();
        for (let i = 0; i < days; i++) {
            const d = new Date(now);
            d.setDate(d.getDate() - i);
            targetDates.push(getLocalDateStr(d));
        }

        let totalSecs = 0;
        let activeDays = 0;
        const appMap = {};
        const hourlyBuckets = new Array(24).fill(0);
        userData.appStatsByDate = userData.appStatsByDate || {};

        targetDates.forEach(dateStr => {
            let dayHasFocus = false;
            const dayStats = userData.appStatsByDate[dateStr];
            if (dayStats) {
                Object.keys(dayStats).forEach(exe => {
                    const sec = dayStats[exe] || 0;
                    if (sec > 0) {
                        totalSecs += sec;
                        appMap[exe] = (appMap[exe] || 0) + sec;
                        dayHasFocus = true;
                    }
                });
            }
            if (userData.hourlyFocusStats && userData.hourlyFocusStats[dateStr]) {
                const dayHours = userData.hourlyFocusStats[dateStr];
                for (let h = 0; h < 24; h++) {
                    if (dayHours[h]) {
                        hourlyBuckets[h] += dayHours[h];
                        if (dayHours[h] > 0) dayHasFocus = true;
                    }
                }
            }
            if (dayHasFocus) activeDays++;
        });

        const sortedApps = Object.keys(appMap)
            .map(exe => ({ exe, secs: appMap[exe] }))
            .sort((a, b) => b.secs - a.secs);

        let productiveSecs = 0;
        let neutralSecs = 0;
        sortedApps.forEach(item => {
            const cat = getAppCategory(item.exe);
            if (cat === 'productive') productiveSecs += item.secs;
            else if (cat === 'neutral') neutralSecs += item.secs;
        });
        const prodScore = totalSecs > 0 ? Math.min(100, Math.round(((productiveSecs * 1.0 + neutralSecs * 0.5) / totalSecs) * 100)) : 100;
        const avgSecs = activeDays > 0 ? Math.round(totalSecs / activeDays) : 0;

        let peakWindowStart = 0;
        let peakWindowSum = 0;
        for (let h = 0; h < 24; h++) {
            const twoHourSum = hourlyBuckets[h] + hourlyBuckets[(h + 1) % 24];
            if (twoHourSum > peakWindowSum) {
                peakWindowSum = twoHourSum;
                peakWindowStart = h;
            }
        }
        const peakWindow = `${String(peakWindowStart).padStart(2, '0')}:00 — ${String((peakWindowStart + 2) % 24).padStart(2, '0')}:00`;
        const chronoInfo = evaluateChronotype(hourlyBuckets, peakWindowStart, Math.floor(totalSecs / 60), (userData.workTimeRangesByDate && userData.workTimeRangesByDate[todayDateStr]));

        let appText = '';
        sortedApps.slice(0, 3).forEach((item, idx) => {
            const percent = totalSecs > 0 ? Math.round((item.secs / totalSecs) * 100) : 0;
            appText += `\n  ${idx + 1}. ${item.exe} — ${formatSecs(item.secs)} (${percent}%)`;
        });

        const spanText = getPeriodMonthSpanText(days);
        const textReport = `🌱 FocusGrow Rhythm & Flow Intelligence Report
📅 Period: ${spanText} (Last ${days} Days — ${days === 30 ? '1 Month' : '3 Months'})
⚔️ Chronotype: ${chronoInfo.persona}
⏱️ Total Focus: ${formatSecs(totalSecs)} across ${activeDays} active days
📊 Daily Average: ${formatSecs(avgSecs)} / active day
⚡ Productivity Score: ${prodScore}%
🔥 Peak Energy Window: ${peakWindow}
🏆 Top Software:${appText || ' None yet'}

#StayFocused #FocusGrow #DeepWork`;

        navigator.clipboard.writeText(textReport).then(() => {
            sendNotification('Report Copied', `${days}-Day Flow report summary copied to clipboard!`, 'focus');
        }).catch(() => {
            sendNotification('Copy Failed', 'Unable to access clipboard.', 'focus');
        });
    });

    // Download High-Resolution Canvas Report Card
    btnDownloadPatternCard?.addEventListener('click', () => {
        const days = currentPatternPeriod;
        const targetDates = [];
        const now = new Date();
        for (let i = 0; i < days; i++) {
            const d = new Date(now);
            d.setDate(d.getDate() - i);
            targetDates.push(getLocalDateStr(d));
        }

        let totalSecs = 0;
        let activeDays = 0;
        const appMap = {};
        const hourlyBuckets = new Array(24).fill(0);
        userData.appStatsByDate = userData.appStatsByDate || {};

        targetDates.forEach(dateStr => {
            let dayHasFocus = false;
            const dayStats = userData.appStatsByDate[dateStr];
            if (dayStats) {
                Object.keys(dayStats).forEach(exe => {
                    const sec = dayStats[exe] || 0;
                    if (sec > 0) {
                        totalSecs += sec;
                        appMap[exe] = (appMap[exe] || 0) + sec;
                        dayHasFocus = true;
                    }
                });
            }
            if (userData.hourlyFocusStats && userData.hourlyFocusStats[dateStr]) {
                const dayHours = userData.hourlyFocusStats[dateStr];
                for (let h = 0; h < 24; h++) {
                    if (dayHours[h]) {
                        hourlyBuckets[h] += dayHours[h];
                        if (dayHours[h] > 0) dayHasFocus = true;
                    }
                }
            }
            if (dayHasFocus) activeDays++;
        });

        const sortedApps = Object.keys(appMap)
            .map(exe => ({ exe, secs: appMap[exe] }))
            .sort((a, b) => b.secs - a.secs);

        let productiveSecs = 0;
        let neutralSecs = 0;
        sortedApps.forEach(item => {
            const cat = getAppCategory(item.exe);
            if (cat === 'productive') productiveSecs += item.secs;
            else if (cat === 'neutral') neutralSecs += item.secs;
        });
        const prodScore = totalSecs > 0 ? Math.min(100, Math.round(((productiveSecs * 1.0 + neutralSecs * 0.5) / totalSecs) * 100)) : 100;
        const avgSecs = activeDays > 0 ? Math.round(totalSecs / activeDays) : 0;

        let peakWindowStart = 0;
        let peakWindowSum = 0;
        for (let h = 0; h < 24; h++) {
            const twoHourSum = hourlyBuckets[h] + hourlyBuckets[(h + 1) % 24];
            if (twoHourSum > peakWindowSum) {
                peakWindowSum = twoHourSum;
                peakWindowStart = h;
            }
        }
        const peakWindow = `${String(peakWindowStart).padStart(2, '0')}:00 — ${String((peakWindowStart + 2) % 24).padStart(2, '0')}:00`;
        const chronoInfo = evaluateChronotype(hourlyBuckets, peakWindowStart, Math.floor(totalSecs / 60), (userData.workTimeRangesByDate && userData.workTimeRangesByDate[todayDateStr]));

        const canvas = document.createElement('canvas');
        const width = 800;
        const height = 620;
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext('2d');

        // Background
        const bgGrad = ctx.createLinearGradient(0, 0, width, height);
        bgGrad.addColorStop(0, '#101624');
        bgGrad.addColorStop(1, '#070b10');
        ctx.fillStyle = bgGrad;
        ctx.fillRect(0, 0, width, height);

        // Ambient glow accents
        const glowTop = ctx.createRadialGradient(width - 40, 40, 10, width - 40, 40, 260);
        glowTop.addColorStop(0, 'rgba(192, 132, 252, 0.28)');
        glowTop.addColorStop(1, 'rgba(192, 132, 252, 0)');
        ctx.fillStyle = glowTop;
        ctx.fillRect(0, 0, width, height);

        const glowBottom = ctx.createRadialGradient(60, height - 60, 10, 60, height - 60, 260);
        glowBottom.addColorStop(0, 'rgba(96, 205, 255, 0.24)');
        glowBottom.addColorStop(1, 'rgba(96, 205, 255, 0)');
        ctx.fillStyle = glowBottom;
        ctx.fillRect(0, 0, width, height);

        // Border
        ctx.strokeStyle = 'rgba(192, 132, 252, 0.4)';
        ctx.lineWidth = 2.5;
        ctx.strokeRect(12, 12, width - 24, height - 24);

        // Header Brand & Title
        ctx.fillStyle = '#60cdff';
        ctx.font = 'bold 26px "Segoe UI", sans-serif';
        ctx.fillText('🌱 FocusGrow', 40, 56);

        ctx.fillStyle = 'rgba(255, 255, 255, 0.55)';
        ctx.font = '13px "Segoe UI", sans-serif';
        ctx.fillText('Focus Rhythm & Chronotype Intelligence', 40, 78);

        // Period Badge on top right
        const cardSpanText = getPeriodMonthSpanText(days);
        const periodStr = `${days === 30 ? '1 MONTH' : '3 MONTHS'} (${cardSpanText.toUpperCase()})`;
        ctx.fillStyle = 'rgba(192, 132, 252, 0.2)';
        ctx.fillRect(width - 320, 36, 280, 34);
        ctx.strokeStyle = 'rgba(192, 132, 252, 0.5)';
        ctx.strokeRect(width - 320, 36, 280, 34);
        ctx.fillStyle = '#d8b4fe';
        ctx.font = 'bold 11px "Segoe UI", sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(periodStr, width - 180, 58);
        ctx.textAlign = 'left';

        // Horizontal separator
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
        ctx.beginPath();
        ctx.moveTo(40, 95);
        ctx.lineTo(width - 40, 95);
        ctx.stroke();

        // 3 KPI Boxes
        const boxWidth = (width - 80 - 24) / 3;
        const boxY = 115;
        const boxH = 75;

        // Box 1: Total Focus
        ctx.fillStyle = 'rgba(255, 255, 255, 0.04)';
        ctx.fillRect(40, boxY, boxWidth, boxH);
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
        ctx.strokeRect(40, boxY, boxWidth, boxH);
        ctx.fillStyle = '#8899a6';
        ctx.font = 'bold 11px "Segoe UI", sans-serif';
        ctx.fillText('TOTAL FOCUS TIME', 55, boxY + 24);
        ctx.fillStyle = '#ffffff';
        ctx.font = 'bold 22px "Segoe UI", sans-serif';
        ctx.fillText(formatSecs(totalSecs), 55, boxY + 54);

        // Box 2: Daily Avg
        const box2X = 40 + boxWidth + 12;
        ctx.fillStyle = 'rgba(255, 255, 255, 0.04)';
        ctx.fillRect(box2X, boxY, boxWidth, boxH);
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
        ctx.strokeRect(box2X, boxY, boxWidth, boxH);
        ctx.fillStyle = '#8899a6';
        ctx.font = 'bold 11px "Segoe UI", sans-serif';
        ctx.fillText('DAILY AVERAGE', box2X + 15, boxY + 24);
        ctx.fillStyle = '#60cdff';
        ctx.font = 'bold 22px "Segoe UI", sans-serif';
        ctx.fillText(formatSecs(avgSecs), box2X + 15, boxY + 54);

        // Box 3: Efficiency & Active Days
        const box3X = box2X + boxWidth + 12;
        ctx.fillStyle = 'rgba(255, 255, 255, 0.04)';
        ctx.fillRect(box3X, boxY, boxWidth, boxH);
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
        ctx.strokeRect(box3X, boxY, boxWidth, boxH);
        ctx.fillStyle = '#8899a6';
        ctx.font = 'bold 11px "Segoe UI", sans-serif';
        ctx.fillText('EFFICIENCY SCORE', box3X + 15, boxY + 24);
        ctx.fillStyle = '#4ade80';
        ctx.font = 'bold 22px "Segoe UI", sans-serif';
        ctx.fillText(`${prodScore}% (${activeDays}d active)`, box3X + 15, boxY + 54);

        // Chronotype Persona Card Banner
        const personaY = 205;
        const personaH = 75;
        ctx.fillStyle = 'rgba(251, 191, 36, 0.12)';
        ctx.fillRect(40, personaY, width - 80, personaH);
        ctx.strokeStyle = chronoInfo.color;
        ctx.lineWidth = 1.5;
        ctx.strokeRect(40, personaY, width - 80, personaH);

        ctx.fillStyle = chronoInfo.color;
        ctx.font = 'bold 18px "Segoe UI", sans-serif';
        ctx.fillText(chronoInfo.persona, 55, personaY + 28);

        ctx.fillStyle = '#ffffff';
        ctx.font = 'bold 13px "Segoe UI", sans-serif';
        ctx.textAlign = 'right';
        ctx.fillText(`Peak Energy: ${peakWindow}`, width - 55, personaY + 28);
        ctx.textAlign = 'left';

        ctx.fillStyle = '#cbd5e1';
        ctx.font = '12px "Segoe UI", sans-serif';
        ctx.fillText(chronoInfo.desc, 55, personaY + 54);

        // 24-Hour Wave Curve
        const waveBoxY = 295;
        const waveBoxH = 110;
        ctx.fillStyle = 'rgba(0, 0, 0, 0.4)';
        ctx.fillRect(40, waveBoxY, width - 80, waveBoxH);
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
        ctx.strokeRect(40, waveBoxY, width - 80, waveBoxH);

        ctx.fillStyle = 'rgba(255, 255, 255, 0.7)';
        ctx.font = 'bold 12px "Segoe UI", sans-serif';
        ctx.fillText('24-HOUR CONCENTRATION FLOW (AVERAGE)', 55, waveBoxY + 22);

        // Wave curve drawing on canvas
        const maxH = Math.max(1, ...hourlyBuckets);
        const waveStartX = 55;
        const waveEndX = width - 55;
        const waveSpan = waveEndX - waveStartX;
        const waveBaseY = waveBoxY + waveBoxH - 22;
        const waveTopY = waveBoxY + 32;

        ctx.strokeStyle = '#60cdff';
        ctx.lineWidth = 2.5;
        ctx.beginPath();
        for (let h = 0; h < 24; h++) {
            const x = waveStartX + (h / 23) * waveSpan;
            const val = hourlyBuckets[h];
            const y = waveBaseY - (maxH > 0 ? (val / maxH) * (waveBaseY - waveTopY) : 0);
            if (h === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.stroke();

        // Wave Labels
        ctx.fillStyle = '#64748b';
        ctx.font = '10px "Segoe UI", sans-serif';
        ctx.fillText('00:00', waveStartX, waveBoxY + waveBoxH - 6);
        ctx.fillText('06:00', waveStartX + waveSpan * 0.25, waveBoxY + waveBoxH - 6);
        ctx.fillText('12:00', waveStartX + waveSpan * 0.50, waveBoxY + waveBoxH - 6);
        ctx.fillText('18:00', waveStartX + waveSpan * 0.75, waveBoxY + waveBoxH - 6);
        ctx.fillText('24:00', waveEndX - 25, waveBoxY + waveBoxH - 6);

        // Top Apps (Bottom Left) & Day-Part Distribution (Bottom Right)
        const bottomY = 420;
        const colW = (width - 80 - 20) / 2;

        // Top Apps
        ctx.fillStyle = '#8899a6';
        ctx.font = 'bold 12px "Segoe UI", sans-serif';
        ctx.fillText('TOP FOCUS SOFTWARE', 40, bottomY + 16);

        const top3 = sortedApps.slice(0, 3);
        let appY = bottomY + 30;
        if (top3.length === 0) {
            ctx.fillStyle = 'rgba(255,255,255,0.4)';
            ctx.font = '12px "Segoe UI", sans-serif';
            ctx.fillText('No focus activity logged.', 40, appY + 15);
        } else {
            top3.forEach((item, idx) => {
                ctx.fillStyle = 'rgba(255, 255, 255, 0.04)';
                ctx.fillRect(40, appY, colW, 30);
                ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
                ctx.strokeRect(40, appY, colW, 30);

                ctx.fillStyle = '#ffffff';
                ctx.font = 'bold 12px "Segoe UI", sans-serif';
                ctx.fillText(`${idx + 1}.  ${item.exe}`, 52, appY + 19);

                const percent = totalSecs > 0 ? Math.round((item.secs / totalSecs) * 100) : 0;
                ctx.fillStyle = '#60cdff';
                ctx.textAlign = 'right';
                ctx.fillText(`${formatSecs(item.secs)} (${percent}%)`, 40 + colW - 12, appY + 19);
                ctx.textAlign = 'left';

                appY += 36;
            });
        }

        // Time Distribution (Bottom Right)
        const distColX = 40 + colW + 20;
        ctx.fillStyle = '#8899a6';
        ctx.font = 'bold 12px "Segoe UI", sans-serif';
        ctx.fillText('TIME OF DAY DISTRIBUTION', distColX, bottomY + 16);

        const morningM = hourlyBuckets.slice(5, 12).reduce((a, b) => a + b, 0);
        const afternoonM = hourlyBuckets.slice(12, 18).reduce((a, b) => a + b, 0);
        const nightM = hourlyBuckets.slice(18, 24).reduce((a, b) => a + b, 0);
        const midnightM = hourlyBuckets.slice(0, 5).reduce((a, b) => a + b, 0);
        const sParts = Math.max(1, morningM + afternoonM + nightM + midnightM);

        const partsData = [
            { label: 'Morning (05-12)', pct: Math.round((morningM / sParts) * 100), color: '#fbbf24' },
            { label: 'Afternoon (12-18)', pct: Math.round((afternoonM / sParts) * 100), color: '#60cdff' },
            { label: 'Night (18-24)', pct: Math.round((nightM / sParts) * 100), color: '#c084fc' }
        ];

        let dY = bottomY + 30;
        partsData.forEach(p => {
            ctx.fillStyle = 'rgba(255, 255, 255, 0.04)';
            ctx.fillRect(distColX, dY, colW, 30);
            ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
            ctx.strokeRect(distColX, dY, colW, 30);

            ctx.fillStyle = '#e2e8f0';
            ctx.font = '12px "Segoe UI", sans-serif';
            ctx.fillText(p.label, distColX + 12, dY + 19);

            ctx.fillStyle = p.color;
            ctx.font = 'bold 13px "Segoe UI", sans-serif';
            ctx.textAlign = 'right';
            ctx.fillText(`${p.pct}%`, distColX + colW - 12, dY + 19);
            ctx.textAlign = 'left';

            dY += 36;
        });

        // Footer
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
        ctx.beginPath();
        ctx.moveTo(40, height - 42);
        ctx.lineTo(width - 40, height - 42);
        ctx.stroke();

        ctx.fillStyle = 'rgba(255, 255, 255, 0.5)';
        ctx.font = 'italic 12px "Segoe UI", sans-serif';
        ctx.fillText('"True mastery comes from long-term rhythm and focused patience."', 40, height - 22);

        ctx.textAlign = 'right';
        ctx.fillStyle = '#60cdff';
        ctx.font = 'bold 11px "Segoe UI", sans-serif';
        ctx.fillText('FocusGrow OS • Windows', width - 40, height - 22);

        // Download PNG
        const link = document.createElement('a');
        link.download = `FocusGrow-Flow-Pattern-Report-${days}d-${todayDateStr}.png`;
        link.href = canvas.toDataURL('image/png');
        link.click();

        sendNotification('Report Downloaded', `Exported ${days}-day flow pattern report as PNG image!`, 'focus');
    });

    // Process C++ IPC Messages & Trigger Notifications
    let _lastUiPipState = null;
    let _lastUiActiveState = null;
    let _lastRenderedPrayerKey = '';
    window.onCppStateUpdate = function (data, isPip) {
        if (!data) return;
        const prevActiveForLayout = _lastUiActiveState;
        const prevPipForLayout = _lastUiPipState;
        if (typeof checkAndHandleMidnightRollover === 'function') {
            checkAndHandleMidnightRollover();
        }
        latestSessionData = data;

        activeState = data.state;
        isPaused = data.isPaused;
        const layoutStateChanged = (prevPipForLayout !== !!isPip) || (prevStateForLayoutChanged(prevActiveForLayout, activeState));
        _lastUiPipState = !!isPip;
        _lastUiActiveState = activeState;

        function prevStateForLayoutChanged(prevS, curS) {
            return prevS !== curS;
        }

        // --- Session Persistence: Save active session metadata ---
        if (activeState !== 'idle') {
            userData.activeSession = {
                state: activeState,
                remainingSec: data.remainingSec,
                currentPeriod: data.currentPeriod,
                totalPeriods: data.totalPeriods,
                focusMins: selectedMins,
                chunkMins: selectedPeriodMins,
                breakMins: selectedBreakMins,
                skipBreaks: chkSkipBreaks.checked,
                isHealthMode: !!data.isHealthMode,
                timestamp: Date.now()
            };
        } else {
            delete userData.activeSession;
        }

        // --- Prayer Times State Sync ---
        const isPrayerBreakActive = !!(data.prayer && data.prayer.isBreakActive);
        const isBreakOrResting = activeState === 'resting' || isPrayerBreakActive;

        if (data.prayer) {
            const prevActualNext = window.actualNextPrayerName;
            window.allPrayerTimes = data.prayer.allTimes || [];
            window.actualNextPrayerName = data.prayer.nextName;

            const isViewingActualNext = window.displayedPrayerIndex !== -1 &&
                window.allPrayerTimes[window.displayedPrayerIndex]?.name === prevActualNext;

            if (window.displayedPrayerIndex === undefined || window.displayedPrayerIndex === -1 || isViewingActualNext) {
                window.displayedPrayerIndex = window.allPrayerTimes.findIndex(p => p.name === window.actualNextPrayerName);
            }

            const prayerKey = `${window.displayedPrayerIndex}|${window.actualNextPrayerName}|${window.allPrayerTimes.length}`;
            if (prayerKey !== _lastRenderedPrayerKey && typeof window.renderPrayerInfo === 'function') {
                _lastRenderedPrayerKey = prayerKey;
                window.renderPrayerInfo();
            }
        }

        if (prevActiveForLayout !== activeState) {
            saveUserData();
        } else {
            saveUserDataDebounced(5000);
        }

        // Record per-app focus time tick (1 sec) & work hours window
        if (activeState === 'focusing' && !isPaused && !isPrayerBreakActive && data.activeExe) {
            userData.appStatsByDate = userData.appStatsByDate || {};
            userData.appStatsByDate[todayDateStr] = userData.appStatsByDate[todayDateStr] || {};
            const currentSec = userData.appStatsByDate[todayDateStr][data.activeExe] || 0;
            userData.appStatsByDate[todayDateStr][data.activeExe] = currentSec + 1;

            // Record Real-Time Hourly Focus Stats (Exact Current Hour Bucket)
            const curH = new Date().getHours();
            userData.hourlyFocusSecs = userData.hourlyFocusSecs || {};
            userData.hourlyFocusSecs[todayDateStr] = userData.hourlyFocusSecs[todayDateStr] || {};
            const curHSec = (userData.hourlyFocusSecs[todayDateStr][curH] || 0) + 1;
            userData.hourlyFocusSecs[todayDateStr][curH] = curHSec;

            userData.hourlyFocusStats = userData.hourlyFocusStats || {};
            userData.hourlyFocusStats[todayDateStr] = userData.hourlyFocusStats[todayDateStr] || {};
            userData.hourlyFocusStats[todayDateStr][curH] = Math.max(1, Math.floor(curHSec / 60));

            const timeNowStr = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
            userData.workTimeRangesByDate = userData.workTimeRangesByDate || {};
            if (!userData.workTimeRangesByDate[todayDateStr]) {
                userData.workTimeRangesByDate[todayDateStr] = { firstStart: timeNowStr, lastActive: timeNowStr };
            } else {
                userData.workTimeRangesByDate[todayDateStr].lastActive = timeNowStr;
            }

            saveUserDataDebounced(5000);
        }

        // Neutral App Overstay Guard & Gentle Distraction Nudge Tracking (Anti-Abuse Recovery Model)
        // If on break (Prayer Break or Rest Break), Paused, or in Posture Care (Health) Mode:
        // User is taking a legitimate rest/break or in relaxed Posture Care mode (gaming, reading, etc.).
        // Do NOT track neutral overstay streak, do NOT send warnings, and reset active streak/soft-block.
        const isHealthModeActive = !!(data.isHealthMode || (userData && userData.sessionMode === 'health'));
        if (isBreakOrResting || isPaused || isHealthModeActive || activeState === 'idle') {
            continuousNeutralSec = 0;
            continuousProductiveSec = 0;
            if (isNeutralSoftBlockActive) {
                isNeutralSoftBlockActive = false;
                sendToCpp({ action: 'hideNeutralOverstayBlock' });
            }
        } else if (data.activeExe) {
            const exeCat = getAppCategory(data.activeExe);
            const lowerExe = data.activeExe.toLowerCase();

            if (exeCat === 'productive') {
                lastProductiveExe = data.activeExe;
                continuousProductiveSec += 1;

                // Temporarily hide soft-block overlay while user is viewing their productive work app
                if (isNeutralSoftBlockActive) {
                    sendToCpp({ action: 'hideNeutralOverstayBlock' });
                }

                // Anti-Abuse: Must sustain at least 60 seconds (1 min) of continuous productive work
                // to officially forgive and reset the neutral/distraction overstay streak
                if (continuousProductiveSec >= 60) {
                    continuousNeutralSec = 0;
                    isNeutralSoftBlockActive = false;
                }
            } else if (exeCat === 'neutral' || exeCat === 'distracting') {
                // Ignore internal and OS shell processes from distraction streak
                if (lowerExe !== 'focusgrow.exe' && lowerExe !== 'dwm.exe' && lowerExe !== 'explorer.exe' && lowerExe !== 'taskmgr.exe') {
                    // Reset productive recovery counter if user left the productive app before reaching 60s
                    continuousProductiveSec = 0;

                    if (lastContinuousExe === data.activeExe) {
                        continuousNeutralSec += 1;
                    } else {
                        lastContinuousExe = data.activeExe;
                        continuousNeutralSec += 1;
                    }

                    if (userData.neutralOverstayEnabled !== false) {
                        const intervalMins = parseInt(userData.neutralOverstayIntervalMins) || 30;
                        const intervalSec = intervalMins * 60;

                        // First Warning: 1x Interval reached (e.g. 15 / 30 mins)
                        if (continuousNeutralSec === intervalSec) {
                            userData.ignoredNudgeStatsByDate = userData.ignoredNudgeStatsByDate || {};
                            userData.ignoredNudgeStatsByDate[todayDateStr] = (userData.ignoredNudgeStatsByDate[todayDateStr] || 0) + 1;
                            saveUserData();
                            if (typeof renderStatsDashboard === 'function') renderStatsDashboard(activeStatsRange);

                            const targetApp = lastProductiveExe || 'your work app';
                            sendNotification(
                                'Neutral App Reminder',
                                `You've been in ${data.activeExe} for ${intervalMins} mins. Ready to return to ${targetApp}?`,
                                'focus'
                            );
                            playNotificationSound('reminder');
                        }
                        // Second Warning or Re-trigger: 2x Interval reached (e.g. 30 / 60 mins) -> Soft-Block!
                        else if (continuousNeutralSec >= intervalSec * 2) {
                            if (!isNeutralSoftBlockActive) {
                                isNeutralSoftBlockActive = true;
                                userData.ignoredNudgeStatsByDate = userData.ignoredNudgeStatsByDate || {};
                                userData.ignoredNudgeStatsByDate[todayDateStr] = (userData.ignoredNudgeStatsByDate[todayDateStr] || 0) + 1;
                                saveUserData();
                                if (typeof renderStatsDashboard === 'function') renderStatsDashboard(activeStatsRange);

                                sendNotification(
                                    'Time to Refocus',
                                    `2x warnings ignored (${Math.floor(continuousNeutralSec / 60)} mins in ${data.activeExe}). Take a step back and resume work!`,
                                    'focus'
                                );
                                playNotificationSound('alarm');
                            }

                            // Ensure soft block overlay is visible if user remains in or returns to neutral app
                            if (userData.neutralSoftBlockEnabled !== false) {
                                sendToCpp({
                                    action: 'showNeutralOverstayBlock',
                                    exeName: data.activeExe,
                                    durationSec: continuousNeutralSec,
                                    durationMins: Math.floor(continuousNeutralSec / 60)
                                });
                            }
                        }
                    }
                }
            }
        }

        // Live Real-Time second-by-second ticker update on the active streak badge in Stats DOM
        function updateLiveStreakInStatsDOM() {
            const statsModal = document.getElementById('stats-modal');
            if (!statsModal || !statsModal.classList.contains('active')) return;

            if (isHealthModeActive || activeState === 'idle') {
                document.querySelectorAll('.live-streak-pill').forEach(el => el.remove());
                return;
            }

            const limitMins = parseInt(userData.neutralOverstayIntervalMins) || 30;
            const liveText = formatLiveStreak(continuousNeutralSec, limitMins);
            const liveMins = Math.floor(continuousNeutralSec / 60);

            // Remove any stale badge on non-active rows
            document.querySelectorAll('.live-streak-pill').forEach(el => {
                const parentRow = el.closest('.stats-app-item');
                const rowExe = parentRow?.getAttribute('data-exe') || '';
                if (!lastContinuousExe || rowExe.toLowerCase() !== lastContinuousExe.toLowerCase() || continuousNeutralSec <= 0) {
                    el.remove();
                }
            });

            if (lastContinuousExe && continuousNeutralSec > 0) {
                const targetLower = lastContinuousExe.toLowerCase();
                const targetCat = getAppCategory(lastContinuousExe);
                if (targetCat !== 'productive') {
                    const row = document.querySelector(`.stats-app-item[data-exe="${targetLower}"]`);
                    if (row) {
                        const nameDiv = row.querySelector('.stats-app-name');
                        if (nameDiv) {
                            let badge = nameDiv.querySelector('.live-streak-pill');
                            if (!badge) {
                                badge = document.createElement('span');
                                badge.className = 'live-streak-pill';
                                badge.id = 'live-overstay-streak-badge';
                                badge.style.cssText = 'background: rgba(96, 205, 255, 0.12); border: 1px solid rgba(96, 205, 255, 0.3); color: #60cdff; font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 4px; display: inline-flex; align-items: center; margin-left: 6px;';
                                nameDiv.appendChild(badge);
                            }
                            badge.textContent = liveText;
                            if (liveMins >= limitMins) {
                                badge.style.background = 'rgba(248, 113, 113, 0.15)';
                                badge.style.borderColor = 'rgba(248, 113, 113, 0.35)';
                                badge.style.color = '#f87171';
                            } else if (liveMins >= limitMins * 0.7) {
                                badge.style.background = 'rgba(251, 146, 60, 0.15)';
                                badge.style.borderColor = 'rgba(251, 146, 60, 0.35)';
                                badge.style.color = '#fb923c';
                            } else {
                                badge.style.background = 'rgba(96, 205, 255, 0.12)';
                                badge.style.borderColor = 'rgba(96, 205, 255, 0.3)';
                                badge.style.color = '#60cdff';
                            }
                        }
                    }
                }
            }
        }
        updateLiveStreakInStatsDOM();

        if (data.localIp && data.localIp !== '127.0.0.1') {
            cachedLocalIp = data.localIp;
        }

        if (data.isHealthMode) {
            document.body.classList.add('session-health-mode');
        } else {
            document.body.classList.remove('session-health-mode');
        }

        if (activeState !== previousState) {
            if (activeState === 'focusing' && previousState === 'idle') {
                if (data.isHealthMode) {
                    sendNotification('Posture Guard Started', `Session started! Relax your shoulders and maintain good posture.`, 'focus');
                } else {
                    sendNotification('Focus Mode Started', `Session started! Period 1 of ${data.totalPeriods || 1}. Stay focused!`, 'focus');
                    updateRandomQuote('focus');
                }
            } else if (activeState === 'resting') {
                if (data.isHealthMode) {
                    sendNotification('Time to Stand & Stretch!', `Session period completed! Stand up, stretch, and hydrate for ${selectedBreakMins} minutes.`, 'break');
                } else {
                    sendNotification('Time to Rest', `Focus period done! Step away from your computer for a ${selectedBreakMins} min break.`, 'break');
                    updateRandomQuote('rest');
                }
            } else if (activeState === 'focusing' && previousState === 'resting') {
                // Record completed stand/posture break across both modes!
                userData.healthCompletedBreaksToday = (userData.healthCompletedBreaksToday || 0) + 1;
                userData.healthStatsByDate = userData.healthStatsByDate || {};
                userData.healthStatsByDate[todayDateStr] = userData.healthStatsByDate[todayDateStr] || {};
                userData.healthStatsByDate[todayDateStr].breaks = userData.healthCompletedBreaksToday;
                userData.healthStreakDays = calculateHealthStreakDays();
                saveUserData();
                updateStatsUI();

                if (data.isHealthMode) {
                    sendNotification('Stand Break Finished', `Great job! You completed your stand break. Ready to resume your session.`, 'focus');
                } else {
                    sendNotification('Focus Mode Active', `Break finished! Return to your work application.`, 'focus');
                    updateRandomQuote('focus');
                }
            } else if (activeState === 'idle' && (previousState === 'focusing' || previousState === 'resting')) {
                if (previousState === 'resting') {
                    userData.healthCompletedBreaksToday = (userData.healthCompletedBreaksToday || 0) + 1;
                    userData.healthStatsByDate = userData.healthStatsByDate || {};
                    userData.healthStatsByDate[todayDateStr] = userData.healthStatsByDate[todayDateStr] || {};
                    userData.healthStatsByDate[todayDateStr].breaks = userData.healthCompletedBreaksToday;
                    userData.healthStreakDays = calculateHealthStreakDays();
                    saveUserData();
                    updateStatsUI();
                }
                if (data.isHealthMode) {
                    sendNotification('Posture Session Completed', `Posture Guard session completed! Great job taking care of your posture.`, 'focus');
                    document.body.classList.remove('session-health-mode');
                } else {
                    sendNotification('Session Completed', `Great job! Focus session completed successfully.`, 'focus');
                    updateRandomQuote('focus');
                }

                // Auto-exit PiP mode when focus session completes, restoring to full main window
                if (document.body.classList.contains('pip-mode')) {
                    isInitializingPip = true;
                    setTimeout(() => { isInitializingPip = false; }, 1500);
                    handlePipTransitionStart(false, 200);
                    sendToCpp({ action: 'togglePip', enabled: false });
                }
            }
                // Transition impact sound when session state changes
                if (previousState === 'focusing' && activeState === 'resting') {
                    playCountdownTransitionImpact('break');
                } else if (previousState === 'resting' && activeState === 'focusing') {
                    playCountdownTransitionImpact('focus');
                }
                window._lastCountdownSec = -1;
            previousState = activeState;
            notifiedOneMinWarning = false;
            if (activeState === 'idle') {
                previousPeriod = 1;
            }
        }

        // Period change detection (e.g. In continuous focus / skip-breaks mode where state remains 'focusing')
        if (data.currentPeriod && data.currentPeriod !== previousPeriod) {
            previousPeriod = data.currentPeriod;
            notifiedOneMinWarning = false;
            window._lastCountdownSec = -1;
            if (activeState === 'focusing' && previousState === 'focusing') {
                playCountdownTransitionImpact('focus');
                if (data.isHealthMode) {
                    sendNotification('Posture Guard Active', `Period ${data.currentPeriod} of ${data.totalPeriods || 1}. Keep your posture relaxed!`, 'focus');
                } else {
                    sendNotification('Focus Period Started', `Period ${data.currentPeriod} of ${data.totalPeriods || 1}. Stay focused!`, 'focus');
                    updateRandomQuote('focus');
                }
            }
        }

        // --- Transition Countdown Sound Effect (Customizable: 3, 5, 10 seconds) ---
        if (userData.countdownSoundEnabled !== false) {
            const cdSec = parseInt(userData.countdownDurationSec) || 3;

            // A. Focus / Rest Session countdown
            if (!isPaused && activeState !== 'idle' && !isPrayerBreakActive) {
                if (data.remainingSec <= cdSec && data.remainingSec >= 1 && window._lastCountdownSec !== data.remainingSec) {
                    window._lastCountdownSec = data.remainingSec;
                    playCountdownPip(data.remainingSec);
                }
            }

            // B. Prayer Break countdown (before prayer break ends -> back to focus)
            if (isPrayerBreakActive && data.prayer && data.prayer.breakRemainingSec !== undefined) {
                const bRem = data.prayer.breakRemainingSec;
                if (bRem <= cdSec && bRem >= 1 && window._lastPrayerCountdownSec !== bRem) {
                    window._lastPrayerCountdownSec = bRem;
                    playCountdownPip(bRem);
                }
            }

            // C. Approaching Prayer Time (before prayer break locks the screen)
            if (!isPrayerBreakActive && data.prayer && data.prayer.secToNext !== undefined && data.prayer.enabled !== false) {
                const pSec = data.prayer.secToNext;
                if (pSec <= cdSec && pSec >= 1 && window._lastPrePrayerCountdownSec !== pSec) {
                    window._lastPrePrayerCountdownSec = pSec;
                    playCountdownPip(pSec);
                }
            }

            // D. Prayer Break transition trigger
            if (!window._wasPrayerBreakActive && isPrayerBreakActive) {
                playCountdownTransitionImpact('break');
                window._lastPrePrayerCountdownSec = -1;
            } else if (window._wasPrayerBreakActive && !isPrayerBreakActive) {
                playCountdownTransitionImpact('focus');
                window._lastPrayerCountdownSec = -1;
            }
            window._wasPrayerBreakActive = isPrayerBreakActive;
        }

        const warnSec = (data.maxPeriodSec <= 60) ? 30 : 60;
        if (data.remainingSec === warnSec && !notifiedOneMinWarning) {
            notifiedOneMinWarning = true;
            const warnText = (data.maxPeriodSec <= 60) ? '30 seconds' : '1 minute';
            const isSkip = (data.skipBreaks !== undefined) ? !!data.skipBreaks : !!chkSkipBreaks.checked;
            if (activeState === 'focusing') {
                if (isSkip) {
                    if (data.currentPeriod >= data.totalPeriods) {
                        sendNotification('Session Ending Soon', `${warnText} left of your focus session! Almost finished.`, 'warning');
                    } else {
                        sendNotification('Period Ending Soon', `${warnText} left of period ${data.currentPeriod}. Continuing directly to next period.`, 'warning');
                    }
                } else {
                    sendNotification(data.isHealthMode ? 'Session Break Warning' : 'Period Warning', `${warnText} left of session! Prepare to stand up & stretch.`, 'warning');
                }
            } else if (activeState === 'resting') {
                sendNotification(data.isHealthMode ? 'Stand Break Warning' : 'Break Warning', `${warnText} left of stand break! Prepare to resume.`, 'warning');
            }
        }

        const titlebarText = document.getElementById('titlebar-text');

        if (!isPipMorphing) {
            if (isPip) {
                document.querySelectorAll('.modal-overlay.active, .modal.active').forEach(m => m.classList.remove('active'));
                document.body.classList.add('pip-mode');
            } else {
                document.body.classList.remove('pip-mode');
            }
            if (layoutStateChanged) {
                updatePipPeekingVinyl();
                requestAnimationFrame(syncVinylCenterPosition);
            }
        }
        if (titlebarText) titlebarText.textContent = 'FocusGrow';

        if (data.formattedTime) {
            activeTimerDisplay.textContent = data.formattedTime;
        }

        // Ignore stale messages from a previous calendar day
        const isFromToday = !data.date || data.date === todayDateStr;
        if (!data.isHealthMode && data.completedMinutes !== undefined && isFromToday) {
            // Guard against stale IPC messages from before midnight:
            // Calculate maximum physically possible minutes elapsed since 00:00 today.
            const now = new Date();
            const maxPossibleMinutesToday = now.getHours() * 60 + now.getMinutes() + 1;

            if (data.completedMinutes > userData.completedMinutesToday) {
                // Only accept if physically plausible for the current calendar day
                if (data.completedMinutes <= maxPossibleMinutesToday) {
                    userData.completedMinutesToday = data.completedMinutes;
                    saveUserData();
                }
            } else if (data.completedMinutes === 0 && userData.completedMinutesToday > 0 && (data.state === 'idle' || Date.now() - lastMidnightRolloverTime < 10000)) {
                userData.completedMinutesToday = 0;
                saveUserData();
            }
        }

        updateStatsUI();

        const autoPauseLabel = document.getElementById('auto-pause-label');

        if (activeState === 'focusing') {
            setupView.classList.remove('active');
            timerView.classList.add('active');
            activeStatusLabel.textContent = isPaused ? 'PAUSED' : (data.isHealthMode ? 'SESSION ACTIVE' : 'REMAINING');
            const isPipCompact = document.body.classList.contains('pip-mode') || window.innerWidth < 260;
            focusPeriodTitle.textContent = data.isHealthMode
                ? (isPipCompact ? `Active (${data.currentPeriod || 1}/${data.totalPeriods || 1})` : `Active Session (${data.currentPeriod || 1} of ${data.totalPeriods || 1})`)
                : (isPipCompact ? `Focus period (${data.currentPeriod || 1}/${data.totalPeriods || 1})` : `Focus period (${data.currentPeriod || 1} of ${data.totalPeriods || 1})`);
            focusPeriodTitle.title = data.isHealthMode
                ? `Active Session (${data.currentPeriod || 1} of ${data.totalPeriods || 1})`
                : `Focus period (${data.currentPeriod || 1} of ${data.totalPeriods || 1})`;
            updateTimerSubtitle(data);

            pauseIcon.style.display = isPaused ? 'none' : 'block';
            playIcon.style.display = isPaused ? 'block' : 'none';

            if (autoPauseLabel) {
                autoPauseLabel.classList.toggle('show', isPaused && data.isAutoPaused);
            }
            if (layoutStateChanged) requestAnimationFrame(syncVinylCenterPosition);
        } else if (activeState === 'resting') {
            setupView.classList.remove('active');
            timerView.classList.add('active');
            activeStatusLabel.textContent = isPaused ? 'PAUSED' : (data.isHealthMode ? 'STAND UP & STRETCH' : 'RESTING & STEP OUTSIDE');
            const isPipCompact = document.body.classList.contains('pip-mode') || window.innerWidth < 260;
            focusPeriodTitle.textContent = data.isHealthMode 
                ? (isPipCompact ? 'Stand-Up Break' : 'Stand-Up Break (Stretch)') 
                : 'Mandatory Break';
            focusPeriodTitle.title = data.isHealthMode ? 'Stand-Up Break (Stretch)' : 'Mandatory Break';
            updateTimerSubtitle(data);

            pauseIcon.style.display = isPaused ? 'none' : 'block';
            playIcon.style.display = isPaused ? 'block' : 'none';

            if (autoPauseLabel) {
                autoPauseLabel.classList.remove('show');
            }
            if (layoutStateChanged) requestAnimationFrame(syncVinylCenterPosition);
        } else {
            timerView.classList.remove('active');
            setupView.classList.add('active');
            document.body.classList.remove('session-health-mode');

            if (autoPauseLabel) {
                autoPauseLabel.classList.remove('show');
            }
            if (layoutStateChanged) requestAnimationFrame(syncVinylCenterPosition);
        }

        if (data.remainingSec !== undefined && data.maxPeriodSec > 0) {
            const ratio = data.remainingSec / data.maxPeriodSec;
            updateVisualProgress(ratio);

            // Real-time Plant Growth Animation (0.0 to 1.0)
            const progressRatio = Math.max(0, Math.min(1.0, 1 - ratio));

            const isTransitionFromRestingToFocus = (window._prevActiveState === 'resting' && activeState === 'focusing');
            const isManualSessionReset = (window._lastPlantProgress !== undefined && window._lastPlantProgress >= 0.90 && progressRatio < 0.15 && activeState === 'focusing');

            if (activeState === 'resting') {
                // Selama Waktu Break / Istirahat: Tanaman TETAP MEKAR PENUH & BERBUNGA EMAS!
                const plantStem = document.getElementById('plant-stem');
                const leaf1 = document.getElementById('leaf-pair-1');
                const leaf2 = document.getElementById('leaf-pair-2');
                const leaf3 = document.getElementById('leaf-pair-3');
                const topBud = document.getElementById('plant-top-bud');
                const flower = document.getElementById('plant-crown-flower');

                if (plantStem) plantStem.style.strokeDashoffset = '0';
                if (leaf1) leaf1.classList.add('grown');
                if (leaf2) leaf2.classList.add('grown');
                if (leaf3) leaf3.classList.add('grown');
                if (topBud) topBud.classList.add('grown');
                if (flower) flower.classList.add('grown');
                window._lastPlantProgress = 1.0;
            } else if (isTransitionFromRestingToFocus || isManualSessionReset) {
                // Saat masuk sesi fokus baru: Jalankan Reverse Shrink kembali ke pot!
                const flower = document.getElementById('plant-crown-flower');
                const topBud = document.getElementById('plant-top-bud');
                const leaf3 = document.getElementById('leaf-pair-3');
                const leaf2 = document.getElementById('leaf-pair-2');
                const leaf1 = document.getElementById('leaf-pair-1');
                const plantStem = document.getElementById('plant-stem');

                // Step A: Bunga menguncup
                if (flower) flower.classList.remove('grown');
                // Step B: Pucuk menguncup
                setTimeout(() => { if (topBud) topBud.classList.remove('grown'); }, 150);
                // Step C: Daun 3 menguncup
                setTimeout(() => { if (leaf3) leaf3.classList.remove('grown'); }, 350);
                // Step D: Daun 2 menguncup
                setTimeout(() => { if (leaf2) leaf2.classList.remove('grown'); }, 550);
                // Step E: Daun 1 menguncup
                setTimeout(() => { if (leaf1) leaf1.classList.remove('grown'); }, 750);
                // Step F: Batang meluncur turun ke dalam pot
                setTimeout(() => { if (plantStem) plantStem.style.strokeDashoffset = '50'; }, 850);

                window._lastPlantProgress = 0.0;
            } else {
                // Real-time Plant Growth strictly matching stem junction heights
                const plantStem = document.getElementById('plant-stem');
                if (plantStem) {
                    plantStem.style.strokeDashoffset = (50 * (1 - progressRatio)).toFixed(1);
                }

                const leaf1 = document.getElementById('leaf-pair-1');
                if (leaf1) leaf1.classList.toggle('grown', progressRatio >= 0.35);

                const leaf2 = document.getElementById('leaf-pair-2');
                if (leaf2) leaf2.classList.toggle('grown', progressRatio >= 0.53);

                const leaf3 = document.getElementById('leaf-pair-3');
                if (leaf3) leaf3.classList.toggle('grown', progressRatio >= 0.84);

                const topBud = document.getElementById('plant-top-bud');
                if (topBud) topBud.classList.toggle('grown', progressRatio >= 0.96);

                const flower = document.getElementById('plant-crown-flower');
                if (flower) flower.classList.toggle('grown', progressRatio >= 0.995);

                window._lastPlantProgress = progressRatio;
            }
            window._prevActiveState = activeState;
        }

        // Trigger Direct Cloud Sync for Android (Standalone, no extension required)
        triggerFirebaseDirectSync(data);
    };

    // --- Direct Firebase Cloud Sync (Standalone Cross-Platform without Extension) ---
    const FIREBASE_RTDB_URL = "https://focusgrow-e2d8f-default-rtdb.asia-southeast1.firebasedatabase.app/sync.json";
    const FIREBASE_GIF_URL = "https://focusgrow-e2d8f-default-rtdb.asia-southeast1.firebasedatabase.app/customGifData.json";

    // Graceful PC Exit: Notify Firebase that app is closed/idle so mobile phone unblocks immediately
    window.addEventListener('beforeunload', () => {
        try {
            const idlePayload = JSON.stringify({
                state: 'idle',
                remainingSec: 0,
                formattedTime: '00:00',
                isPaused: false,
                isAutoPaused: false,
                lastUpdate: Date.now()
            });
            if (navigator.sendBeacon) {
                navigator.sendBeacon(FIREBASE_RTDB_URL, new Blob([idlePayload], { type: 'application/json' }));
            }
        } catch (e) {}
    });
    let lastUploadedGifHash = null;
    let lastDirectFirebasePush = 0;
    let lastPushedState = null;
    let isPushingFirebase = false;

    function computeSimpleHash(str) {
        if (!str) return '';
        let hash = 0;
        const len = str.length;
        const sampleLimit = Math.min(len, 1000);
        for (let i = 0; i < sampleLimit; i++) {
            hash = ((hash << 5) - hash) + str.charCodeAt(i);
            hash |= 0;
        }
        return 'gif_' + Math.abs(hash) + '_' + len;
    }

    async function triggerFirebaseDirectSync(data, force = false) {
        if (!data) return;
        const now = Date.now();
        const isPrayerBreak = !!(data.prayer && data.prayer.isBreakActive);
        const stateChanged = lastPushedState !== data.state || (isPrayerBreak !== (window._lastPrayerBreakActive || false));
        window._lastPrayerBreakActive = isPrayerBreak;

        const minInterval = (data.state === 'focusing' || data.state === 'resting' || isPrayerBreak) ? 3500 : 10000;
        if (!force && !stateChanged && (now - lastDirectFirebasePush < minInterval)) {
            return;
        }

        if (isPushingFirebase) return;
        isPushingFirebase = true;
        lastDirectFirebasePush = now;
        lastPushedState = data.state;

        try {
            const passesMap = {};
            let hasLocalActivePass = false;
            if (Array.isArray(data.activePasses)) {
                data.activePasses.forEach(p => {
                    if (p && p.domain && p.remainingSec > 0) {
                        const k = p.domain.replace(/\./g, '_');
                        passesMap[k] = {
                            domain: p.domain,
                            remainingSec: p.remainingSec,
                            targetEndTime: Date.now() + (p.remainingSec * 1000),
                            passesLeft: p.passesLeft !== undefined ? p.passesLeft : 0,
                            lastGrantTime: Date.now()
                        };
                        hasLocalActivePass = true;
                    }
                });
            }

            const payload = {
                state: data.state || 'idle',
                formattedTime: data.formattedTime || '00:00',
                remainingSec: data.remainingSec || 0,
                maxPeriodSec: data.maxPeriodSec || 1,
                currentPeriod: data.currentPeriod || 1,
                totalPeriods: data.totalPeriods || 1,
                completedMinutes: (data.completedMinutes !== undefined && data.completedMinutes > 0) ? data.completedMinutes : (userData.completedMinutesToday || 0),
                dailyGoalHours: parseFloat(userData.dailyGoalHours || 1),
                yesterdayHours: parseFloat(userData.yesterdayHours || 0),
                streakDays: parseInt(userData.streakDays || 0),
                isPaused: !!data.isPaused,
                isAutoPaused: !!data.isAutoPaused,
                activeExe: data.activeExe || '',
                activeDomain: data.activeDomain || '',
                prayer: data.prayer || null,
                lastUpdate: now
            };

            // STRICT: Only include activePasses if we actually have active passes locally
            // Otherwise, never send activePasses in root PATCH so cloud passes from HP are preserved!
            if (hasLocalActivePass) {
                payload.activePasses = passesMap;
            }

            // Include active ambient GIF hash (Ultra-efficient 1-time payload sync)
            try {
                let rawGif = userData.customGifData || '';
                // If stored in cardGifImg or gaugeGifImg, use it directly
                if (!rawGif || (!rawGif.startsWith('data:') && !rawGif.startsWith('http'))) {
                    if (cardGifImg && cardGifImg.src && (cardGifImg.src.startsWith('data:') || cardGifImg.src.startsWith('http'))) {
                        rawGif = cardGifImg.src;
                    } else if (gaugeGifImg && gaugeGifImg.src && (gaugeGifImg.src.startsWith('data:') || gaugeGifImg.src.startsWith('http'))) {
                        rawGif = gaugeGifImg.src;
                    }
                }
                // Fallback to IndexedDB lookup if still an ID
                if (rawGif && !rawGif.startsWith('data:') && !rawGif.startsWith('http')) {
                    const db = window.gifDb || gifDb;
                    if (db && typeof db.get === 'function') {
                        const fetched = await db.get(rawGif);
                        if (fetched) rawGif = fetched;
                    }
                }
                if (rawGif && (rawGif.startsWith('http') || (rawGif.startsWith('data:') && rawGif.length < 10000000))) {
                    const currentHash = computeSimpleHash(rawGif);
                    payload.gifHash = currentHash;
                    payload.gifOpacity = (userData.gifOpacity || 78) / 100;

                    // Upload full heavy GIF data to dedicated node ONCE only when changed
                    if (lastUploadedGifHash !== currentHash) {
                        fetch(FIREBASE_GIF_URL, {
                            method: 'PUT',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ data: rawGif, hash: currentHash })
                        }).then(() => {
                            lastUploadedGifHash = currentHash;
                        }).catch(e => console.warn('[Sync] Failed uploading custom GIF to cloud:', e));
                    }
                } else if (!userData.customGifData || userData.ambientMode === 'plant') {
                    payload.gifHash = "";
                    if (lastUploadedGifHash !== "") {
                        fetch(FIREBASE_GIF_URL, {
                            method: 'PUT',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify(null)
                        }).then(() => {
                            lastUploadedGifHash = "";
                        }).catch(() => {});
                    }
                }
                // Clear legacy multi-megabyte customGif property from root Firebase RTDB state
                payload.customGif = null;
            } catch (gifErr) {
                console.warn('[Sync] Error resolving custom gif:', gifErr);
            }

            await fetch(FIREBASE_RTDB_URL, {
                method: 'PATCH',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            // Periodically check if HP granted an active pass via Firebase
            checkCloudPassesForDesktop();
        } catch (e) {
            // Silently ignore network failures when offline
        } finally {
            isPushingFirebase = false;
        }
    }

    // Check Firebase for passes granted by phone so C++ Desktop engine knows
    let lastCloudPassCheck = 0;
    async function checkCloudPassesForDesktop() {
        if (activeState === 'idle') return;
        const now = Date.now();
        if (now - lastCloudPassCheck < 5000) return;
        lastCloudPassCheck = now;
        try {
            const res = await fetch(FIREBASE_RTDB_URL);
            if (!res.ok) return;
            const cloudData = await res.json();
            if (cloudData && cloudData.activePasses) {
                for (const k in cloudData.activePasses) {
                    const pass = cloudData.activePasses[k];
                    if (pass && pass.domain && pass.remainingSec > 0) {
                        const grantTime = pass.lastGrantTime || 0;
                        if (now - grantTime < 7200 * 1000) {
                            const currentC = (latestSessionData && Array.isArray(latestSessionData.activePasses))
                                ? latestSessionData.activePasses.find(x => x.domain === pass.domain)
                                : null;
                            if (!currentC || currentC.remainingSec <= 0) {
                                sendToCpp({
                                    action: 'grantPass',
                                    domain: pass.domain,
                                    minutes: Math.max(1, Math.ceil(pass.remainingSec / 60))
                                });
                            }
                        }
                    }
                }
            }
        } catch (e) {}
    }
    setInterval(checkCloudPassesForDesktop, 6000);

    function syncCurrentStateToFirebase(force = false) {
        const fallback = latestSessionData || {
            state: activeState || 'idle',
            formattedTime: activeTimerDisplay ? activeTimerDisplay.textContent : '00:00',
            remainingSec: 0,
            maxPeriodSec: (selectedPeriodMins || 25) * 60,
            currentPeriod: 1,
            totalPeriods: 1,
            completedMinutes: userData.completedMinutesToday || 0,
            dailyGoalHours: parseFloat(userData.dailyGoalHours || 1),
            yesterdayHours: parseFloat(userData.yesterdayHours || 0),
            streakDays: parseInt(userData.streakDays || 0),
            isPaused: false
        };
        triggerFirebaseDirectSync(fallback, force);
    }
    window.syncCurrentStateToFirebase = syncCurrentStateToFirebase;

    function sendToCpp(payload) {
        if (window.chrome && window.chrome.webview) {
            window.chrome.webview.postMessage(JSON.stringify(payload));
        } else {
            console.log('C++ IPC bridge (Standalone mode):', payload);
        }
    }

    if (window.chrome && window.chrome.webview) {
        window.chrome.webview.addEventListener('message', event => {
            try {
                const msg = typeof event.data === 'string' ? JSON.parse(event.data) : event.data;
                if (msg.type === 'stateUpdate') {
                    window.onCppStateUpdate(msg.data, msg.isPip);
                } else if (msg.type === 'pipTransitionStart') {
                    if (typeof handlePipTransitionStart === 'function') {
                        handlePipTransitionStart(msg.entering, msg.duration);
                    }
                } else if (msg.type === 'pipTransitionEnd') {
                    if (typeof handlePipTransitionEnd === 'function') {
                        handlePipTransitionEnd(msg.isPip);
                    }
                } else if (msg.type === 'runningApps') {
                    window.renderAppList(msg.apps);
                } else if (msg.type === 'windowBlur') {
                    handleWindowBlur();
                } else if (msg.type === 'windowFocus') {
                    handleWindowFocus();
                } else if (msg.type === 'pipResized') {
                    if (msg.width >= 150 && msg.width <= 440 && msg.height >= 210 && msg.height <= 550) {
                        userData.pipWidth = msg.width;
                        userData.pipHeight = msg.height;
                        saveUserData();
                        console.log(`[PIP] C++ Resized Sync: ${msg.width}x${msg.height}`);
                    }
                } else if (msg.type === 'systemResume') {
                    if (typeof checkAndHandleMidnightRollover === 'function') {
                        checkAndHandleMidnightRollover();
                    }
                    updateStatsUI();
                }
            } catch (err) {
                console.error('IPC JSON parse error:', err);
            }
        });
    }

    updatePickerDisplay();
    updateStatsUI();
    updateAutoPipToggleUI();

    // Sync initial PiP dimensions to C++
    sendToCpp({
        action: 'syncPipSize',
        width: userData.pipWidth || 195,
        height: userData.pipHeight || 280
    });

    // Sync initial stats to C++
    sendToCpp({
        action: 'syncStats',
        completedMinutesToday: userData.completedMinutesToday || 0,
        dailyGoalMinutes: (userData.dailyGoalHours || 1) * 60,
        streakDays: parseInt(userData.streakDays || 0),
        yesterdayHours: parseFloat(userData.yesterdayHours || 0)
    });

    // Sync initial auto-pause config to C++
    sendToCpp({
        action: 'setAutoPauseConfig',
        enabled: userData.autoPauseEnabled !== false,
        sec: userData.autoPauseSec || 15
    });

    // --- Session Persistence: Resume session if one was active ---
    if (userData.activeSession && userData.activeSession.state !== 'idle') {
        const s = userData.activeSession;
        // Optional: only resume if it was less than 2 hours ago?
        const twoHours = 2 * 60 * 60 * 1000;
        if (Date.now() - s.timestamp < twoHours) {
            sendToCpp({
                action: 'resumeSession',
                sessionState: s.state,
                remainingSec: s.remainingSec,
                currentPeriod: s.currentPeriod,
                totalPeriods: s.totalPeriods,
                focusMinutes: s.focusMins,
                focusChunkMinutes: s.chunkMins,
                breakMinutes: s.breakMins,
                skipBreaks: s.skipBreaks,
                isHealthMode: !!s.isHealthMode
            });
        } else {
            delete userData.activeSession;
            saveUserData();
        }
    }

    sendToCpp({ action: 'getRunningApps' });

    // --- Prayer Times UI Feature ---
    const prayerClockDisplay = document.getElementById('prayer-digital-clock');
    const prayerDateDisplay = document.getElementById('prayer-today-date');
    const prayerNextName = document.getElementById('prayer-next-name-val');
    const prayerNextTime = document.getElementById('prayer-next-time-val');

    const chkPrayerEnabled = document.getElementById('chk-prayer-enabled');
    const chkPrayerBreakEnabled = document.getElementById('chk-prayer-break-enabled');
    const inputPrayerAdvance = document.getElementById('input-prayer-advance');
    const inputPrayerLat = document.getElementById('input-prayer-lat');
    const inputPrayerLng = document.getElementById('input-prayer-lng');
    const inputPrayerTz = document.getElementById('input-prayer-tz');
    const btnSavePrayerConfig = document.getElementById('btn-save-prayer-config');

    const btnPrayerGps = document.getElementById('btn-prayer-gps');
    const inputCitySearch = document.getElementById('input-prayer-city-search');
    const btnCitySearch = document.getElementById('btn-prayer-city-search');

    window.allPrayerTimes = [];
    window.displayedPrayerIndex = -1;
    window.actualNextPrayerName = "";

    window.renderPrayerInfo = function () {
        if (window.displayedPrayerIndex === -1 || !window.allPrayerTimes.length) return;
        const p = window.allPrayerTimes[window.displayedPrayerIndex];
        const label = document.getElementById('prayer-display-label');
        if (label) {
            label.textContent = (p.name === window.actualNextPrayerName) ? "Next Prayer" : "Prayer Time";
            label.style.color = (p.name === window.actualNextPrayerName) ? "var(--accent-blue)" : "var(--text-muted)";
        }
        const nameEl = document.getElementById('prayer-next-name-val');
        const timeEl = document.getElementById('prayer-next-time-val');
        if (nameEl) {
            nameEl.textContent = p.name;
            nameEl.style.cursor = "pointer";
            nameEl.title = "Click to reset to Next Prayer";
        }
        if (timeEl) timeEl.textContent = p.time;
    };

    document.getElementById('btn-prayer-prev')?.addEventListener('click', (e) => {
        e.stopPropagation();
        if (!window.allPrayerTimes.length) return;
        window.displayedPrayerIndex = (window.displayedPrayerIndex - 1 + window.allPrayerTimes.length) % window.allPrayerTimes.length;
        window.renderPrayerInfo();
    });

    document.getElementById('btn-prayer-next')?.addEventListener('click', (e) => {
        e.stopPropagation();
        if (!window.allPrayerTimes.length) return;
        window.displayedPrayerIndex = (window.displayedPrayerIndex + 1) % window.allPrayerTimes.length;
        window.renderPrayerInfo();
    });

    document.getElementById('prayer-next-name-val')?.addEventListener('click', () => {
        window.displayedPrayerIndex = window.allPrayerTimes.findIndex(p => p.name === window.actualNextPrayerName);
        window.renderPrayerInfo();
    });

    if (chkPrayerEnabled) {
        chkPrayerEnabled.checked = !!userData.prayerEnabled;
        chkPrayerBreakEnabled.checked = !!userData.prayerBreakEnabled;
        inputPrayerAdvance.value = userData.prayerAdvance || 5;
        inputPrayerLat.value = userData.prayerLat || -2.8554;
        inputPrayerLng.value = userData.prayerLng || 115.3283;
        inputPrayerTz.value = userData.prayerTz || 8;

        // Initial sync to Native
        sendToCpp({
            action: 'setPrayerConfig',
            enabled: userData.prayerEnabled,
            breakEnabled: userData.prayerBreakEnabled,
            advance: userData.prayerAdvance,
            breakDuration: 15,
            lat: userData.prayerLat,
            lng: userData.prayerLng,
            tz: userData.prayerTz
        });
    }

    function savePrayerSettings() {
        if (!chkPrayerEnabled) return;
        userData.prayerEnabled = chkPrayerEnabled.checked;
        userData.prayerBreakEnabled = chkPrayerBreakEnabled.checked;
        userData.prayerAdvance = parseInt(inputPrayerAdvance.value);
        userData.prayerLat = parseFloat(inputPrayerLat.value);
        userData.prayerLng = parseFloat(inputPrayerLng.value);
        userData.prayerTz = parseInt(inputPrayerTz.value);
        saveUserData();

        sendToCpp({
            action: 'setPrayerConfig',
            enabled: userData.prayerEnabled,
            breakEnabled: userData.prayerBreakEnabled,
            advance: userData.prayerAdvance,
            breakDuration: 15,
            lat: userData.prayerLat,
            lng: userData.prayerLng,
            tz: userData.prayerTz
        });
    }

    [chkPrayerEnabled, chkPrayerBreakEnabled, inputPrayerAdvance, inputPrayerLat, inputPrayerLng, inputPrayerTz].forEach(el => {
        el?.addEventListener('change', savePrayerSettings);
    });

    btnPrayerGps?.addEventListener('click', () => {
        if (!navigator.geolocation) {
            alert("Geolocation is not supported by this browser.");
            return;
        }
        btnPrayerGps.innerHTML = '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="spin-icon"><path d="M21 12a9 9 0 1 1-6.219-8.56"></path></svg> Locating...';
        navigator.geolocation.getCurrentPosition((pos) => {
            const lat = pos.coords.latitude;
            const lng = pos.coords.longitude;
            inputPrayerLat.value = lat.toFixed(6);
            inputPrayerLng.value = lng.toFixed(6);

            // Auto-Guess Timezone based on Longitude
            if (lng < 109) inputPrayerTz.value = 7;
            else if (lng < 126) inputPrayerTz.value = 8;
            else inputPrayerTz.value = 9;

            if (inputCitySearch) inputCitySearch.value = ""; // Clear search box to avoid confusion
            btnPrayerGps.innerHTML = '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="#10b981" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg> Success';
            savePrayerSettings();
            setTimeout(() => btnPrayerGps.innerHTML = '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/></svg> Auto GPS', 2000);
        }, (err) => {
            alert("Failed to get GPS location. Please try searching for your city instead.");
            btnPrayerGps.innerHTML = '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="#f87171" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg> Failed';
        });
    });

    const cityResultsDiv = document.getElementById('prayer-city-results');
    const PRAYER_SEARCH_SVG = '<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>';
    const PRAYER_SEARCH_SPIN = '<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="spin-icon"><path d="M21 12a9 9 0 1 1-6.219-8.56"></path></svg>';

    async function searchCity(query) {
        if (!query) {
            if (cityResultsDiv) cityResultsDiv.style.display = 'none';
            return;
        }
        btnCitySearch.innerHTML = PRAYER_SEARCH_SPIN;
        try {
            // Increase limit and add countrycodes=id to prioritize Indonesia
            const resp = await fetch(`https://nominatim.openstreetmap.org/search?format=json&q=${encodeURIComponent(query)}&limit=10&countrycodes=id`);
            const data = await resp.json();

            if (data && data.length > 0) {
                if (cityResultsDiv) {
                    cityResultsDiv.innerHTML = '';
                    cityResultsDiv.style.display = 'block';

                    data.forEach(item => {
                        const div = document.createElement('div');
                        div.style.padding = '8px 10px';
                        div.style.fontSize = '10px';
                        div.style.color = '#eee';
                        div.style.cursor = 'pointer';
                        div.style.borderBottom = '1px solid rgba(255,255,255,0.05)';
                        div.style.transition = 'background 0.2s';
                        div.textContent = item.display_name;

                        div.onmouseover = () => div.style.background = 'rgba(255,255,255,0.1)';
                        div.onmouseout = () => div.style.background = 'transparent';

                        div.onclick = () => {
                            const lat = parseFloat(item.lat);
                            const lng = parseFloat(item.lon);
                            inputPrayerLat.value = lat.toFixed(6);
                            inputPrayerLng.value = lng.toFixed(6);

                            if (lng < 109) inputPrayerTz.value = 7;
                            else if (lng < 126) inputPrayerTz.value = 8;
                            else inputPrayerTz.value = 9;

                            savePrayerSettings();
                            cityResultsDiv.style.display = 'none';
                            inputCitySearch.value = item.display_name.split(',')[0];
                        };
                        cityResultsDiv.appendChild(div);
                    });
                }
                btnCitySearch.innerHTML = PRAYER_SEARCH_SVG;
            } else {
                alert("City not found. Try with a more specific name.");
                btnCitySearch.innerHTML = PRAYER_SEARCH_SVG;
                if (cityResultsDiv) cityResultsDiv.style.display = 'none';
            }
        } catch (e) {
            console.error("Search failed", e);
            btnCitySearch.innerHTML = PRAYER_SEARCH_SVG;
            if (cityResultsDiv) cityResultsDiv.style.display = 'none';
        }
    }

    // Close dropdown when clicking outside
    document.addEventListener('click', (e) => {
        if (cityResultsDiv && !cityResultsDiv.contains(e.target) && e.target !== btnCitySearch && e.target !== inputCitySearch) {
            cityResultsDiv.style.display = 'none';
        }
    });

    btnCitySearch?.addEventListener('click', () => searchCity(inputCitySearch.value));
    inputCitySearch?.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') searchCity(inputCitySearch.value);
    });

    // ========================================================
    // High-Fidelity Natural Ambient DSP Synthesis Engines
    // Acoustically engineered for rich, warm, non-fatiguing sound
    // ========================================================

    class PinkNoiseGen {
        constructor() {
            this.b0 = 0; this.b1 = 0; this.b2 = 0; this.b3 = 0; this.b4 = 0; this.b5 = 0; this.b6 = 0;
        }
        next() {
            const white = Math.random() * 2 - 1;
            this.b0 = 0.99886 * this.b0 + white * 0.0555179;
            this.b1 = 0.99332 * this.b1 + white * 0.0750759;
            this.b2 = 0.96900 * this.b2 + white * 0.1538520;
            this.b3 = 0.86650 * this.b3 + white * 0.3104856;
            this.b4 = 0.55000 * this.b4 + white * 0.5329522;
            this.b5 = -0.7616 * this.b5 - white * 0.0168980;
            const out = (this.b0 + this.b1 + this.b2 + this.b3 + this.b4 + this.b5 + this.b6 + white * 0.5362) * 0.11;
            this.b6 = white * 0.115926;
            return out;
        }
    }

    class BiquadFilter {
        constructor(type = 'lowpass', freq = 1000, Q = 0.707, sampleRate = 44100, gainDb = 0) {
            this.sampleRate = sampleRate;
            this.x1 = 0; this.x2 = 0;
            this.y1 = 0; this.y2 = 0;
            this.setParams(type, freq, Q, gainDb);
        }
        setParams(type, freq, Q = 0.707, gainDb = 0) {
            const w0 = 2 * Math.PI * Math.min(Math.max(freq, 20), this.sampleRate * 0.48) / this.sampleRate;
            const cosw0 = Math.cos(w0);
            const sinw0 = Math.sin(w0);
            const alpha = sinw0 / (2 * Math.max(0.01, Q));
            let b0 = 0, b1 = 0, b2 = 0, a0 = 1, a1 = 0, a2 = 0;

            if (type === 'lowpass') {
                b0 = (1 - cosw0) / 2;
                b1 = 1 - cosw0;
                b2 = (1 - cosw0) / 2;
                a0 = 1 + alpha;
                a1 = -2 * cosw0;
                a2 = 1 - alpha;
            } else if (type === 'highpass') {
                b0 = (1 + cosw0) / 2;
                b1 = -(1 + cosw0);
                b2 = (1 + cosw0) / 2;
                a0 = 1 + alpha;
                a1 = -2 * cosw0;
                a2 = 1 - alpha;
            } else if (type === 'bandpass') {
                b0 = alpha;
                b1 = 0;
                b2 = -alpha;
                a0 = 1 + alpha;
                a1 = -2 * cosw0;
                a2 = 1 - alpha;
            } else if (type === 'peaking') {
                const A = Math.pow(10, gainDb / 40);
                b0 = 1 + alpha * A;
                b1 = -2 * cosw0;
                b2 = 1 - alpha * A;
                a0 = 1 + alpha / A;
                a1 = -2 * cosw0;
                a2 = 1 - alpha / A;
            }

            this.b0 = b0 / a0;
            this.b1 = b1 / a0;
            this.b2 = b2 / a0;
            this.a1 = a1 / a0;
            this.a2 = a2 / a0;
        }
        process(input) {
            const out = this.b0 * input + this.b1 * this.x1 + this.b2 * this.x2 - this.a1 * this.y1 - this.a2 * this.y2;
            this.x2 = this.x1;
            this.x1 = input;
            this.y2 = this.y1;
            this.y1 = (Math.abs(out) < 1e-15) ? 0 : out;
            return this.y1;
        }
    }

    // ========================================================
    // 1. Natural Light Rain & Soothing Window Drizzle
    // 100% Noise & Friction Based (ZERO sine waves, zero metallic ringing)
    // Soft, cozy, velvety rain wash with delicate raindrop pats on leaves/glass
    // ========================================================
    class NaturalLightRainDSP {
        constructor(sampleRate = 44100) {
            this.sampleRate = sampleRate;
            this.pinkL = new PinkNoiseGen();
            this.pinkR = new PinkNoiseGen();

            // Smooth Butterworth filters - ZERO peaking / ZERO resonant boost (eliminates all hollow hum/tinny ring!)
            this.lpL = new BiquadFilter('lowpass', 2400, 0.707, sampleRate);
            this.lpR = new BiquadFilter('lowpass', 2400, 0.707, sampleRate);
            this.hpL = new BiquadFilter('highpass', 180, 0.707, sampleRate);
            this.hpR = new BiquadFilter('highpass', 180, 0.707, sampleRate);

            // Subtle second gentle lowpass to create that cozy, warm indoor rain feel
            this.warmLpL = new BiquadFilter('lowpass', 3600, 0.5, sampleRate);
            this.warmLpR = new BiquadFilter('lowpass', 3600, 0.5, sampleRate);

            // NO SINE OSCILLATORS!
            // Raindrops are subtle micro-bursts of soft friction noise (brushing/tapping)
            this.maxDrops = 14;
            this.drops = [];
            for (let i = 0; i < this.maxDrops; i++) {
                this.drops.push({
                    active: false,
                    age: 0,
                    maxAge: 0.01,
                    decayRate: 300,
                    gain: 0.08,
                    panL: 0.5,
                    panR: 0.5,
                    filter: new BiquadFilter('bandpass', 3200, 0.9, sampleRate)
                });
            }

            this.lfoPhase = Math.random() * Math.PI * 2;
            this.dropTimer = 0;
            this.nextDropInterval = Math.floor(sampleRate / (10 + Math.random() * 12));
        }

        compute(length, out0, out1) {
            const dt = 1.0 / this.sampleRate;
            const lfoInc = (2 * Math.PI * 0.04) / this.sampleRate;

            for (let i = 0; i < length; i++) {
                this.lfoPhase += lfoInc;
                if (this.lfoPhase > Math.PI * 2) this.lfoPhase -= Math.PI * 2;
                const breath = 0.96 + 0.04 * Math.sin(this.lfoPhase);

                // Smooth velvety rain bed
                const rawL = this.pinkL.next();
                const rawR = this.pinkR.next();
                const bedL = this.warmLpL.process(this.hpL.process(this.lpL.process(rawL))) * 0.42 * breath;
                const bedR = this.warmLpR.process(this.hpR.process(this.lpR.process(rawR))) * 0.42 * breath;

                // Soft noise-based raindrops (micro friction pats, ZERO sine tones!)
                this.dropTimer++;
                if (this.dropTimer >= this.nextDropInterval) {
                    this.dropTimer = 0;
                    const v = this.drops.find(d => !d.active);
                    if (v) {
                        v.active = true;
                        v.age = 0;
                        v.maxAge = 0.006 + Math.random() * 0.008; // 6ms - 14ms (very short!)
                        v.decayRate = 350 + Math.random() * 200;
                        v.gain = 0.08 + Math.random() * 0.12;
                        const f = 2600 + Math.random() * 1800; // soft high frequency water pat
                        v.filter.setParams('bandpass', f, 1.0);
                        const pan = (Math.random() * 1.6 - 0.8);
                        v.panL = (1 - pan) * 0.5;
                        v.panR = (1 + pan) * 0.5;
                    }
                    this.nextDropInterval = Math.floor(this.sampleRate / (10 + Math.random() * 16));
                }

                let dropL = 0;
                let dropR = 0;
                for (let d = 0; d < this.maxDrops; d++) {
                    const drop = this.drops[d];
                    if (!drop.active) continue;

                    // Micro white noise burst shaped by fast decay
                    const white = Math.random() * 2 - 1;
                    const env = Math.exp(-drop.decayRate * drop.age);
                    const shaped = drop.filter.process(white) * env * drop.gain;

                    dropL += shaped * drop.panL;
                    dropR += shaped * drop.panR;

                    drop.age += dt;
                    if (drop.age >= drop.maxAge) {
                        drop.active = false;
                    }
                }

                out0[i] = (bedL + dropL) * 0.95;
                if (out1) out1[i] = (bedR + dropR) * 0.95;
            }
        }
    }

    // ========================================================
    // 2. Natural Heavy Rain & Atmospheric Downpour
    // Dynamic storm fluctuations ("bunyi hujan tidak menentu")
    // Deep ground rumble + cascading sheet + heavy splatters
    // 100% noise-based, zero synthetic ringing
    // ========================================================
    class NaturalHeavyRainDSP {
        constructor(sampleRate = 44100) {
            this.sampleRate = sampleRate;
            this.pinkL = new PinkNoiseGen();
            this.pinkR = new PinkNoiseGen();

            // Ground wash & deep bass rumble
            this.brownL = 0;
            this.brownR = 0;
            this.rumbleLpL = new BiquadFilter('lowpass', 200, 0.707, sampleRate);
            this.rumbleLpR = new BiquadFilter('lowpass', 200, 0.707, sampleRate);

            // Mid cascading downpour body
            this.midBpL = new BiquadFilter('bandpass', 1250, 0.6, sampleRate);
            this.midBpR = new BiquadFilter('bandpass', 1250, 0.6, sampleRate);

            // High spray & air mist
            this.sprayLpL = new BiquadFilter('lowpass', 3800, 0.6, sampleRate);
            this.sprayLpR = new BiquadFilter('lowpass', 3800, 0.6, sampleRate);
            this.sprayHpL = new BiquadFilter('highpass', 2000, 0.6, sampleRate);
            this.sprayHpR = new BiquadFilter('highpass', 2000, 0.6, sampleRate);

            // Compound storm fluctuation LFOs (Natural non-repeating surge cycles)
            this.lfo1 = Math.random() * Math.PI * 2;
            this.lfo2 = Math.random() * Math.PI * 2;
            this.lfo3 = Math.random() * Math.PI * 2;

            // Heavy Splatters Pool (Noise burst water slaps, ZERO sine tones!)
            this.maxSplatters = 16;
            this.splatters = [];
            for (let i = 0; i < this.maxSplatters; i++) {
                this.splatters.push({
                    active: false,
                    age: 0,
                    maxAge: 0.02,
                    decayRate: 150,
                    gain: 0.15,
                    panL: 0.5,
                    panR: 0.5,
                    filter: new BiquadFilter('bandpass', 1600, 0.8, sampleRate)
                });
            }
            this.splatterTimer = 0;
            this.nextSplatterInterval = Math.floor(sampleRate / 50);
        }

        compute(length, out0, out1) {
            const dt = 1.0 / this.sampleRate;
            const inc1 = (2 * Math.PI * 0.055) / this.sampleRate;
            const inc2 = (2 * Math.PI * 0.118) / this.sampleRate;
            const inc3 = (2 * Math.PI * 0.238) / this.sampleRate;

            for (let i = 0; i < length; i++) {
                // Compound storm dynamics ("bunyi hujan tidak menentu")
                this.lfo1 += inc1; if (this.lfo1 > Math.PI * 2) this.lfo1 -= Math.PI * 2;
                this.lfo2 += inc2; if (this.lfo2 > Math.PI * 2) this.lfo2 -= Math.PI * 2;
                this.lfo3 += inc3; if (this.lfo3 > Math.PI * 2) this.lfo3 -= Math.PI * 2;

                const stormSwell = 0.76 + 0.15 * Math.sin(this.lfo1) + 0.10 * Math.sin(this.lfo2 + 0.5) + 0.06 * Math.sin(this.lfo3 + 1.2);

                const nL = this.pinkL.next();
                const nR = this.pinkR.next();

                // 1. Deep ground wash & heavy rumble
                this.brownL = (this.brownL + (0.025 * (Math.random() * 2 - 1))) / 1.025;
                this.brownR = (this.brownR + (0.025 * (Math.random() * 2 - 1))) / 1.025;
                const rumbleL = this.rumbleLpL.process(this.brownL * 3.5) * (0.32 * stormSwell);
                const rumbleR = this.rumbleLpR.process(this.brownR * 3.5) * (0.32 * stormSwell);

                // 2. Mid cascading downpour body
                const midL = this.midBpL.process(nL) * (0.45 * stormSwell);
                const midR = this.midBpR.process(nR) * (0.45 * stormSwell);

                // 3. Dense atmospheric mist spray
                const sprayL = this.sprayLpL.process(this.sprayHpL.process(nL)) * (0.22 * stormSwell);
                const sprayR = this.sprayLpR.process(this.sprayHpR.process(nR)) * (0.22 * stormSwell);

                // Heavy splatter drops (noise bursts)
                this.splatterTimer++;
                if (this.splatterTimer >= this.nextSplatterInterval) {
                    this.splatterTimer = 0;
                    const v = this.splatters.find(s => !s.active);
                    if (v) {
                        v.active = true;
                        v.age = 0;
                        v.maxAge = 0.01 + Math.random() * 0.015;
                        v.decayRate = 180 + Math.random() * 120;
                        v.gain = (0.08 + Math.random() * 0.12) * stormSwell;
                        const f = 1200 + Math.random() * 1200;
                        v.filter.setParams('bandpass', f, 0.9);
                        const pan = (Math.random() * 1.8 - 0.9);
                        v.panL = (1 - pan) * 0.5;
                        v.panR = (1 + pan) * 0.5;
                    }
                    const rate = 40 + stormSwell * 40;
                    this.nextSplatterInterval = Math.floor(this.sampleRate / rate);
                }

                let splatL = 0;
                let splatR = 0;
                for (let s = 0; s < this.maxSplatters; s++) {
                    const splat = this.splatters[s];
                    if (!splat.active) continue;

                    const white = Math.random() * 2 - 1;
                    const env = Math.exp(-splat.decayRate * splat.age);
                    const val = splat.filter.process(white) * env * splat.gain;

                    splatL += val * splat.panL;
                    splatR += val * splat.panR;

                    splat.age += dt;
                    if (splat.age >= splat.maxAge) {
                        splat.active = false;
                    }
                }

                out0[i] = (rumbleL + midL + sprayL + splatL) * 0.85;
                if (out1) out1[i] = (rumbleR + midR + sprayR + splatR) * 0.85;
            }
        }
    }

    // ========================================================
    // 3. Natural Campfire & Hearth Fire (Full Stereo)
    // 100% NOISE & TRANSIENT BASED (ZERO sine waves, zero "kaleng dipentung"!)
    // Warm low combustion draft + gentle ember sizzle + dry wood splinter snaps
    // ========================================================
    class NaturalCampfireDSP {
        constructor(sampleRate = 44100) {
            this.sampleRate = sampleRate;
            this.pinkL = new PinkNoiseGen();
            this.pinkR = new PinkNoiseGen();

            // Warm combustion draft (gentle air convection rumble, smooth lowpass 150Hz, Q=0.707)
            this.draftLpL = new BiquadFilter('lowpass', 150, 0.707, sampleRate);
            this.draftLpR = new BiquadFilter('lowpass', 150, 0.707, sampleRate);
            this.flickerPhase = Math.random() * Math.PI * 2;

            // Continuous gentle ember sizzle bed (soft sap boiling, bandpass 1400-2400Hz)
            this.sizzleBpL = new BiquadFilter('bandpass', 1800, 0.8, sampleRate);
            this.sizzleBpR = new BiquadFilter('bandpass', 1800, 0.8, sampleRate);

            // Wood Snaps & Crackles (100% NOISE & SPLINTER BASED - NO SINE WAVES, NO KALENG DIPENTUNG!)
            this.maxPops = 12;
            this.pops = [];
            for (let i = 0; i < this.maxPops; i++) {
                this.pops.push({
                    active: false,
                    age: 0,
                    maxAge: 0.02,
                    decayRate: 200,
                    gain: 0.2,
                    panL: 0.5,
                    panR: 0.5,
                    filter: new BiquadFilter('bandpass', 1800, 1.2, sampleRate)
                });
            }

            this.popTimer = 0;
            this.nextPopInterval = Math.floor(sampleRate * (0.3 + Math.random() * 0.8));
        }

        compute(length, out0, out1) {
            const dt = 1.0 / this.sampleRate;
            const flickerInc = (2 * Math.PI * 1.2) / this.sampleRate;

            for (let i = 0; i < length; i++) {
                this.flickerPhase += flickerInc;
                if (this.flickerPhase > Math.PI * 2) this.flickerPhase -= Math.PI * 2;
                const flicker = 0.88 + 0.12 * Math.sin(this.flickerPhase);

                const nL = this.pinkL.next();
                const nR = this.pinkR.next();

                // Low-end combustion air warmth (soft draft, NO resonant boom)
                const draftL = this.draftLpL.process(nL) * 0.38 * flicker;
                const draftR = this.draftLpR.process(nR) * 0.38 * flicker;

                // Soft continuous ember sizzle (delicate boiling sap in embers)
                const sizzleL = this.sizzleBpL.process(nL) * 0.14 * (0.85 + 0.15 * Math.sin(this.flickerPhase * 2.8));
                const sizzleR = this.sizzleBpR.process(nR) * 0.14 * (0.85 + 0.15 * Math.sin(this.flickerPhase * 2.8));

                // Wood splinter snaps & ember crackles
                this.popTimer++;
                if (this.popTimer >= this.nextPopInterval) {
                    this.popTimer = 0;

                    // Clustered natural fire pops: 1 to 3 tiny snaps
                    const burst = Math.random() < 0.3 ? (2 + Math.floor(Math.random() * 2)) : 1;
                    for (let b = 0; b < burst; b++) {
                        const v = this.pops.find(p => !p.active);
                        if (v) {
                            v.active = true;
                            v.age = 0;
                            // Real dry wood snaps are ultra-fast: 6ms to 18ms
                            v.maxAge = 0.006 + Math.random() * 0.014;
                            v.decayRate = 250 + Math.random() * 250;
                            v.gain = 0.15 + Math.random() * 0.25;

                            // Randomize filter frequency for each snap (dry twig crackle: 1200Hz - 3200Hz)
                            const f = 1100 + Math.random() * 2200;
                            v.filter.setParams('bandpass', f, 1.4);

                            const pan = (Math.random() * 1.6 - 0.8);
                            v.panL = (1 - pan) * 0.5;
                            v.panR = (1 + pan) * 0.5;
                        }
                    }

                    // Irregular pause: occasional quiet glowing pauses of 1.2s - 3.5s
                    const isLongPause = Math.random() < 0.35;
                    const pauseSec = isLongPause ? (1.2 + Math.random() * 2.5) : (0.15 + Math.random() * 0.5);
                    this.nextPopInterval = Math.floor(this.sampleRate * pauseSec);
                }

                // Render active wood snaps (noise-based splinter transients, ZERO sine waves!)
                let popL = 0;
                let popR = 0;
                for (let p = 0; p < this.maxPops; p++) {
                    const pop = this.pops[p];
                    if (!pop.active) continue;

                    const white = Math.random() * 2 - 1;
                    const env = Math.exp(-pop.decayRate * pop.age);
                    const snap = pop.filter.process(white) * env * pop.gain;

                    popL += snap * pop.panL;
                    popR += snap * pop.panR;

                    pop.age += dt;
                    if (pop.age >= pop.maxAge) {
                        pop.active = false;
                    }
                }

                out0[i] = (draftL + sizzleL + popL) * 0.95;
                if (out1) out1[i] = (draftR + sizzleR + popR) * 0.95;
            }
        }
    }

    // ========================================================
    // 4. Natural Field Crickets & Peaceful Night Garden
    // Classic peaceful chirps, natural long pauses, ZERO alien frequencies!
    // ========================================================
    class NaturalCricketDSP {
        constructor(sampleRate = 44100) {
            this.sampleRate = sampleRate;

            // Soft nocturnal night breeze bed (warm brown noise < 220Hz)
            this.pink = new PinkNoiseGen();
            this.airLp = new BiquadFilter('lowpass', 220, 0.707, sampleRate);

            // Natural Field Crickets (Sporadic gentle chirps, peaceful pauses)
            this.c1 = {
                phase: 0,
                baseFreq: 4520,
                curFreq: 4520,
                panL: 0.65, panR: 0.35,
                gain: 0.32,
                state: 'idle',
                syllableCount: 0,
                targetSyllables: 3,
                timeInState: 0,
                nextChirpDelay: 0.8
            };

            this.c2 = {
                phase: 0,
                baseFreq: 4740,
                curFreq: 4740,
                panL: 0.30, panR: 0.70,
                gain: 0.22,
                state: 'idle',
                syllableCount: 0,
                targetSyllables: 2,
                timeInState: 0,
                nextChirpDelay: 1.8
            };
        }

        updateCricket(c, dt) {
            c.timeInState += dt;

            if (c.state === 'idle') {
                if (c.timeInState >= c.nextChirpDelay) {
                    c.state = 'syllable';
                    c.timeInState = 0;
                    c.syllableCount = 0;
                    c.targetSyllables = 2 + Math.floor(Math.random() * 2); // 2 to 3 gentle chirps
                    c.curFreq = c.baseFreq + (Math.random() * 60 - 30);
                }
                return 0;
            }

            const syllableDur = 0.022; // 22ms gentle chirp
            const gapDur = 0.024;      // 24ms silent gap

            if (c.state === 'syllable') {
                if (c.timeInState >= syllableDur) {
                    c.syllableCount++;
                    if (c.syllableCount >= c.targetSyllables) {
                        c.state = 'idle';
                        c.timeInState = 0;
                        // Long, peaceful natural pauses between chirping phrases (1.5s - 3.8s)
                        c.nextChirpDelay = 1.5 + Math.random() * 2.3;
                    } else {
                        c.state = 'gap';
                        c.timeInState = 0;
                    }
                    return 0;
                }

                c.phase += (2 * Math.PI * c.curFreq) * dt;
                // Hann envelope (smooth, zero clicks, soft attack & release)
                const env = 0.5 * (1.0 - Math.cos((2 * Math.PI * c.timeInState) / syllableDur));
                return Math.sin(c.phase) * env * c.gain;
            }

            if (c.state === 'gap') {
                if (c.timeInState >= gapDur) {
                    c.state = 'syllable';
                    c.timeInState = 0;
                }
                return 0;
            }

            return 0;
        }

        compute(length, out0, out1) {
            const dt = 1.0 / this.sampleRate;

            for (let i = 0; i < length; i++) {
                // Soft nocturnal air cushion
                const air = this.airLp.process(this.pink.next()) * 0.15;

                // Gentle sporadic field crickets (natural rhythm)
                const s1 = this.updateCricket(this.c1, dt);
                const s2 = this.updateCricket(this.c2, dt);

                const cL = (s1 * this.c1.panL + s2 * this.c2.panL);
                const cR = (s1 * this.c1.panR + s2 * this.c2.panR);

                out0[i] = (cL + air) * 0.85;
                if (out1) out1[i] = (cR + air) * 0.85;
            }
        }
    }

    // Aliases for compatibility
    const FaustRainDSP = NaturalLightRainDSP;
    const FaustHeavyRainDSP = NaturalHeavyRainDSP;
    const FaustFireDSP = NaturalCampfireDSP;
    const FaustCricketDSP = NaturalCricketDSP;

    // ========================================================
    // Continuous Dynamic Audio Streamer
    // Computes sample-by-sample audio on the fly in sequential chunks.
    // Preserves PRNG & DSP state across chunks so sound NEVER loops or repeats!
    // ========================================================
    class ContinuousAudioStreamer {
        constructor(ctx, gainNode, dsp, chunkDuration = 4, isStereo = false) {
            this.ctx = ctx;
            this.gainNode = gainNode;
            this.dsp = dsp;
            this.chunkDuration = chunkDuration;
            this.isStereo = isStereo;
            this.isRunning = false;
            this.timer = null;
            this.nextStartTime = 0;
            this.activeSources = [];
        }

        start() {
            if (this.isRunning) return;
            this.isRunning = true;
            this.nextStartTime = this.ctx.currentTime + 0.05;
            this.scheduleNextChunk();
            this.scheduleNextChunk();
            this.checkSchedule();
        }

        scheduleNextChunk() {
            if (!this.isRunning || !this.ctx) return;
            const sampleRate = this.ctx.sampleRate || 44100;
            const length = Math.floor(sampleRate * this.chunkDuration);
            const buffer = this.ctx.createBuffer(this.isStereo ? 2 : 1, length, sampleRate);
            const out0 = buffer.getChannelData(0);
            const out1 = this.isStereo ? buffer.getChannelData(1) : null;

            if (this.isStereo) {
                this.dsp.compute(length, out0, out1);
            } else {
                this.dsp.compute(length, out0);
            }

            const source = this.ctx.createBufferSource();
            source.buffer = buffer;
            source.connect(this.gainNode);

            const startTime = Math.max(this.ctx.currentTime + 0.01, this.nextStartTime);
            source.start(startTime);
            this.nextStartTime = startTime + this.chunkDuration;

            this.activeSources.push(source);
            source.onended = () => {
                const idx = this.activeSources.indexOf(source);
                if (idx !== -1) this.activeSources.splice(idx, 1);
            };
        }

        checkSchedule() {
            if (!this.isRunning) return;
            if (this.nextStartTime - this.ctx.currentTime < 5.0) {
                this.scheduleNextChunk();
            }
            this.timer = setTimeout(() => this.checkSchedule(), 1200);
        }

        stop() {
            this.isRunning = false;
            if (this.timer) clearTimeout(this.timer);
            this.timer = null;
            for (const src of this.activeSources) {
                try { src.stop(); src.disconnect(); } catch (e) { }
            }
            this.activeSources = [];
        }
    }
    const FaustStreamer = ContinuousAudioStreamer;

    // ========================================================
    // Procedural Web Audio Ambient Soundscapes Engine
    // ========================================================
    class AmbientAudioEngine {
        constructor() {
            this.ctx = null;
            this.masterGain = null;
            this.tracks = {};
            this.isPlaying = false;
            this.isMasterMuted = false;
            this.typingTimer = null;

            const defaultTracks = {
                typing: { active: true, volume: 65 },
                fire: { active: false, volume: 60 },
                cricket: { active: true, volume: 50 },
                rain_drizzle: { active: false, volume: 55 },
                rain_heavy: { active: false, volume: 50 },
                night: { active: false, volume: 45 },
                clock: { active: false, volume: 50 },
                ocean: { active: false, volume: 45 },
                alpha: { active: false, volume: 30 }
            };

            this.settings = userData.ambientSettings || {
                enabled: true,
                autoPlay: true,
                masterVolume: 80,
                activePreset: 'night_coder',
                tracks: defaultTracks
            };
            if (!this.settings.tracks) this.settings.tracks = {};
            // Migration for legacy vinyl track to wall clock
            if (this.settings.tracks.vinyl) {
                if (!this.settings.tracks.clock) {
                    this.settings.tracks.clock = { active: this.settings.tracks.vinyl.active, volume: this.settings.tracks.vinyl.volume || 50 };
                }
                delete this.settings.tracks.vinyl;
            }
            // Migration for legacy rain setting
            if (this.settings.tracks.rain) {
                if (!this.settings.tracks.rain_drizzle) {
                    this.settings.tracks.rain_drizzle = { active: this.settings.tracks.rain.active, volume: this.settings.tracks.rain.volume || 55 };
                }
                if (!this.settings.tracks.rain_heavy) {
                    this.settings.tracks.rain_heavy = { active: false, volume: 50 };
                }
                delete this.settings.tracks.rain;
            }
            for (const key in defaultTracks) {
                if (!this.settings.tracks[key]) {
                    this.settings.tracks[key] = { ...defaultTracks[key] };
                }
            }
        }

        initContext() {
            if (!this.ctx) {
                const AudioContextClass = window.AudioContext || window.webkitAudioContext;
                this.ctx = new AudioContextClass();
                this.masterGain = this.ctx.createGain();
                this.masterGain.gain.setValueAtTime((this.settings.masterVolume / 100) * (this.isMasterMuted ? 0 : 1), this.ctx.currentTime);
                this.masterGain.connect(this.ctx.destination);
            }
            if (this.ctx.state === 'suspended') {
                this.ctx.resume();
            }
        }

        createNoiseBuffer(type = 'pink', duration = 5) {
            const sampleRate = this.ctx ? this.ctx.sampleRate : 44100;
            const bufferSize = sampleRate * duration;
            const buffer = this.ctx.createBuffer(1, bufferSize, sampleRate);
            const data = buffer.getChannelData(0);

            if (type === 'white') {
                for (let i = 0; i < bufferSize; i++) {
                    data[i] = Math.random() * 2 - 1;
                }
            } else if (type === 'pink') {
                let b0 = 0, b1 = 0, b2 = 0, b3 = 0, b4 = 0, b5 = 0, b6 = 0;
                for (let i = 0; i < bufferSize; i++) {
                    const white = Math.random() * 2 - 1;
                    b0 = 0.99886 * b0 + white * 0.0555179;
                    b1 = 0.99332 * b1 + white * 0.0750759;
                    b2 = 0.96900 * b2 + white * 0.1538520;
                    b3 = 0.86650 * b3 + white * 0.3104856;
                    b4 = 0.55000 * b4 + white * 0.5329522;
                    b5 = -0.7616 * b5 - white * 0.0168980;
                    data[i] = (b0 + b1 + b2 + b3 + b4 + b5 + b6 + white * 0.5362) * 0.11;
                    b6 = white * 0.115926;
                }
            } else if (type === 'brown') {
                let lastOut = 0.0;
                for (let i = 0; i < bufferSize; i++) {
                    const white = Math.random() * 2 - 1;
                    data[i] = (lastOut + (0.02 * white)) / 1.02;
                    lastOut = data[i];
                    data[i] *= 3.5;
                }
            }
            return buffer;
        }

        createMechanicalKeyBuffer(isSpace = false) {
            const sampleRate = this.ctx ? this.ctx.sampleRate : 44100;
            const length = Math.floor(sampleRate * (isSpace ? 0.038 : 0.024));
            const buffer = this.ctx.createBuffer(1, length, sampleRate);
            const data = buffer.getChannelData(0);

            // Creamy Lubed Mechanical Switch ASMR (Warm, soothing, zero harsh piercing spikes)
            let prev = 0;
            let smooth = 0;
            const decay = isSpace ? 120 : 190;

            for (let i = 0; i < length; i++) {
                const t = i / sampleRate;
                const white = (Math.random() * 2 - 1);

                // Smooth 2ms attack to eliminate any sharp ear-piercing click
                const attack = Math.min(1, t / 0.0022);
                const env = attack * Math.exp(-t * decay);

                // Soft tactile friction
                smooth = smooth * 0.7 + white * 0.3;
                const tactile = smooth * env * 1.6;

                // Warm keycap body tap
                const tap = (white - prev) * Math.exp(-t * 900) * 0.85;
                prev = white;

                data[i] = (tactile * 0.9 + tap * 0.35) * (0.85 + Math.random() * 0.25);
            }
            return buffer;
        }

        createMouseClickBuffer(isUpstroke = false) {
            const sampleRate = this.ctx ? this.ctx.sampleRate : 44100;
            const length = Math.floor(sampleRate * (isUpstroke ? 0.012 : 0.018));
            const buffer = this.ctx.createBuffer(1, length, sampleRate);
            const data = buffer.getChannelData(0);

            // Tactile Microswitch Mouse Click (Omron switch acoustic profile)
            let prev = 0;
            const decay = isUpstroke ? 750 : 480;
            for (let i = 0; i < length; i++) {
                const t = i / sampleRate;
                const white = (Math.random() * 2 - 1);
                const click = (white - prev) * Math.exp(-t * decay) * (isUpstroke ? 0.8 : 1.5);
                prev = white;
                data[i] = click * 0.7;
            }
            return buffer;
        }

        playMouseClick(isDoubleClick = false) {
            if (!this.tracks.typing || !this.tracks.typing.active || !this.isPlaying || this.isMasterMuted) return;
            const now = this.ctx.currentTime;

            const playClick = (timeOffset = 0) => {
                const clickTime = now + timeOffset;
                const src = this.ctx.createBufferSource();
                src.buffer = this.createMouseClickBuffer(false);

                const filter = this.ctx.createBiquadFilter();
                filter.type = 'bandpass';
                filter.frequency.setValueAtTime(2350 + Math.random() * 300, clickTime);
                filter.Q.setValueAtTime(1.4, clickTime);

                const gain = this.ctx.createGain();
                gain.gain.setValueAtTime(0.42 + Math.random() * 0.16, clickTime);

                src.connect(filter);
                filter.connect(gain);
                gain.connect(this.tracks.typing.gainNode);
                src.start(clickTime);

                // Upstroke release click (~22ms later)
                const upSrc = this.ctx.createBufferSource();
                upSrc.buffer = this.createMouseClickBuffer(true);
                upSrc.connect(filter);
                upSrc.start(clickTime + 0.022);
            };

            playClick(0);
            if (isDoubleClick) {
                playClick(0.095);
            }
        }

        startTypingTrack() {
            if (this.tracks.typing) return;
            const gainNode = this.ctx.createGain();
            gainNode.gain.setValueAtTime((this.settings.tracks.typing.volume / 100), this.ctx.currentTime);
            gainNode.connect(this.masterGain);

            this.tracks.typing = { gainNode, active: true };

            const playSingleKeystroke = (isSpace = false) => {
                if (!this.tracks.typing || !this.tracks.typing.active || !this.isPlaying || this.isMasterMuted) return;
                const now = this.ctx.currentTime;

                const source = this.ctx.createBufferSource();
                source.buffer = this.createMechanicalKeyBuffer(isSpace);
                source.playbackRate.setValueAtTime(isSpace ? 0.88 : (0.94 + Math.random() * 0.14), now);

                // Warm Low-pass filter (removes any harsh/ear-piercing frequencies above 1500Hz)
                const filter = this.ctx.createBiquadFilter();
                filter.type = 'lowpass';
                filter.frequency.setValueAtTime(isSpace ? 1100 : (1450 + Math.random() * 250), now);
                filter.Q.setValueAtTime(0.7, now);

                const keyGain = this.ctx.createGain();
                const vel = (0.5 + Math.random() * 0.3) * (isSpace ? 1.2 : 1.0);
                keyGain.gain.setValueAtTime(vel, now);

                source.connect(filter);
                filter.connect(keyGain);
                keyGain.connect(gainNode);

                source.start(now);
            };

            const scheduleTypingBurst = () => {
                if (!this.tracks.typing || !this.isPlaying) return;

                // 25% chance of doing mouse actions instead of typing
                if (Math.random() < 0.25) {
                    const isDouble = Math.random() < 0.35;
                    this.playMouseClick(isDouble);
                    const nextActionDelay = 500 + Math.random() * 900;
                    this.typingTimer = setTimeout(scheduleTypingBurst, nextActionDelay);
                    return;
                }

                // Random keystrokes count (from 3 to 12 strokes per burst)
                const keystrokesCount = 3 + Math.floor(Math.random() * 10);
                let delay = 0;

                for (let i = 0; i < keystrokesCount; i++) {
                    const isSpace = (i === keystrokesCount - 1) || (Math.random() < 0.18);
                    setTimeout(() => {
                        playSingleKeystroke(isSpace);
                    }, delay);
                    // Realistic typing speed: 70ms - 150ms per keystroke (human cadence)
                    delay += 70 + Math.floor(Math.random() * 80);
                }

                // Occasional mouse click immediately after typing a line (35% chance)
                if (Math.random() < 0.35) {
                    setTimeout(() => {
                        this.playMouseClick(Math.random() < 0.3);
                    }, delay + 120 + Math.floor(Math.random() * 200));
                }

                // Natural human thinking pause between words & sentences (0.4s - 2.2s)
                const nextBurstDelay = delay + (Math.random() < 0.3 ? (1100 + Math.random() * 1200) : (380 + Math.random() * 600));
                this.typingTimer = setTimeout(scheduleTypingBurst, nextBurstDelay);
            };

            scheduleTypingBurst();
        }

        stopTypingTrack() {
            if (this.typingTimer) clearTimeout(this.typingTimer);
            if (this.tracks.typing) {
                delete this.tracks.typing;
            }
        }

        startFireTrack() {
            if (this.tracks.fire) return;
            const gainNode = this.ctx.createGain();
            const baseVolume = (this.settings.tracks.fire.volume / 100);
            gainNode.gain.setValueAtTime(baseVolume, this.ctx.currentTime);
            gainNode.connect(this.masterGain);

            // Natural Stereo Campfire (Warm draft, gentle ember sizzle, non-repetitive wood pops)
            const sampleRate = this.ctx.sampleRate || 44100;
            const dsp = new NaturalCampfireDSP(sampleRate);
            const streamer = new ContinuousAudioStreamer(this.ctx, gainNode, dsp, 4, true);
            streamer.start();

            this.tracks.fire = { gainNode, streamer, active: true };
        }

        stopFireTrack() {
            if (this.tracks.fire) {
                if (this.tracks.fire.streamer) this.tracks.fire.streamer.stop();
                delete this.tracks.fire;
            }
        }

        startCricketTrack() {
            if (this.tracks.cricket) return;
            const gainNode = this.ctx.createGain();
            const baseVolume = (this.settings.tracks.cricket.volume / 100);
            gainNode.gain.setValueAtTime(baseVolume, this.ctx.currentTime);
            gainNode.connect(this.masterGain);

            // Natural Stereo Crickets + Suara Tonggeret Shimmer (Warm nocturnal soundscape)
            const sampleRate = this.ctx.sampleRate || 44100;
            const dsp = new NaturalCricketDSP(sampleRate);
            const streamer = new ContinuousAudioStreamer(this.ctx, gainNode, dsp, 4, true);
            streamer.start();

            this.tracks.cricket = { gainNode, streamer, active: true };
        }

        stopCricketTrack() {
            if (this.tracks.cricket) {
                if (this.tracks.cricket.streamer) this.tracks.cricket.streamer.stop();
                delete this.tracks.cricket;
            }
        }

        startRainDrizzleTrack() {
            if (this.tracks.rain_drizzle) return;
            const gainNode = this.ctx.createGain();
            const baseVolume = (this.settings.tracks.rain_drizzle.volume / 100);
            gainNode.gain.setValueAtTime(baseVolume, this.ctx.currentTime);
            gainNode.connect(this.masterGain);

            // Natural Stereo Light Rain (Soft drizzle bed + delicate raindrops on glass/leaves)
            const sampleRate = this.ctx.sampleRate || 44100;
            const dsp = new NaturalLightRainDSP(sampleRate);
            const streamer = new ContinuousAudioStreamer(this.ctx, gainNode, dsp, 4, true);
            streamer.start();

            this.tracks.rain_drizzle = { gainNode, streamer, active: true };
        }

        stopRainDrizzleTrack() {
            if (this.tracks.rain_drizzle) {
                if (this.tracks.rain_drizzle.streamer) this.tracks.rain_drizzle.streamer.stop();
                delete this.tracks.rain_drizzle;
            }
        }

        startRainHeavyTrack() {
            if (this.tracks.rain_heavy) return;
            const gainNode = this.ctx.createGain();
            const baseVolume = (this.settings.tracks.rain_heavy.volume / 100);
            gainNode.gain.setValueAtTime(baseVolume, this.ctx.currentTime);
            gainNode.connect(this.masterGain);

            // Natural Stereo Heavy Rain ("Bunyi hujan tidak menentu" with dynamic storm surges)
            const sampleRate = this.ctx.sampleRate || 44100;
            const dsp = new NaturalHeavyRainDSP(sampleRate);
            const streamer = new ContinuousAudioStreamer(this.ctx, gainNode, dsp, 4, true);
            streamer.start();

            this.tracks.rain_heavy = { gainNode, streamer, active: true };
        }

        stopRainHeavyTrack() {
            if (this.tracks.rain_heavy) {
                if (this.tracks.rain_heavy.streamer) this.tracks.rain_heavy.streamer.stop();
                delete this.tracks.rain_heavy;
            }
        }

        startNightTrack() {
            if (this.tracks.night) return;
            const gainNode = this.ctx.createGain();
            gainNode.gain.setValueAtTime(this.settings.tracks.night.volume / 100, this.ctx.currentTime);
            gainNode.connect(this.masterGain);

            const windNoise = this.ctx.createBufferSource();
            windNoise.buffer = this.createNoiseBuffer('brown', 6);
            windNoise.loop = true;

            const windFilter = this.ctx.createBiquadFilter();
            windFilter.type = 'lowpass';
            windFilter.frequency.setValueAtTime(260, this.ctx.currentTime);

            const windGain = this.ctx.createGain();
            windGain.gain.setValueAtTime(0.32, this.ctx.currentTime);

            // Gentle nocturnal wind wave swell (0.06Hz)
            const windLfo = this.ctx.createOscillator();
            windLfo.frequency.setValueAtTime(0.06, this.ctx.currentTime);
            const windLfoGain = this.ctx.createGain();
            windLfoGain.gain.setValueAtTime(0.08, this.ctx.currentTime);
            windLfo.connect(windLfoGain);
            windLfoGain.connect(windGain.gain);
            windLfo.start();

            windNoise.connect(windFilter);
            windFilter.connect(windGain);
            windGain.connect(gainNode);
            windNoise.start();

            this.tracks.night = { gainNode, nodes: [windNoise, windLfo], active: true };
        }

        stopNightTrack() {
            if (this.tracks.night) {
                if (this.tracks.night.nodes) {
                    this.tracks.night.nodes.forEach(n => {
                        try { n.stop(); } catch (e) { }
                    });
                }
                delete this.tracks.night;
            }
        }

        getClockTickBuffer(isTock = false) {
            if (isTock && this._tockBuffer) return this._tockBuffer;
            if (!isTock && this._tickBuffer) return this._tickBuffer;

            const sampleRate = this.ctx ? this.ctx.sampleRate : 44100;
            // 95ms captures direct mechanical strike plus natural room reflections across the room
            const duration = 0.095;
            const length = Math.floor(sampleRate * duration);
            const buffer = this.ctx.createBuffer(2, length, sampleRate); // True Stereo Spatial Buffer
            const outL = buffer.getChannelData(0);
            const outR = buffer.getChannelData(1);

            // True Acoustic Physical Resonator (Zero sine oscillators)
            const createBP = (freq, Q) => {
                const w0 = 2 * Math.PI * freq / sampleRate;
                const alpha = Math.sin(w0) / (2 * Q);
                const cosw = Math.cos(w0);
                const a0 = 1 + alpha;
                return {
                    b0: alpha / a0,
                    b2: -alpha / a0,
                    a1: -2 * cosw / a0,
                    a2: (1 - alpha) / a0,
                    x1: 0, x2: 0, y1: 0, y2: 0,
                    process(x) {
                        const y = this.b0 * x + this.b2 * this.x2 - this.a1 * this.y1 - this.a2 * this.y2;
                        this.x2 = this.x1; this.x1 = x;
                        this.y2 = this.y1; this.y1 = y;
                        return y;
                    }
                };
            };

            // Natural wood and brass movement modes:
            // Tick (crisp wood housing & escapement) vs Tock (deeper hollow cabinet)
            const f1 = isTock ? 450 : 660;
            const f2 = isTock ? 920 : 1260;
            const f3 = isTock ? 1480 : 1820;

            const bp1 = createBP(f1, 1.4);
            const bp2 = createBP(f2, 2.0);
            const bp3 = createBP(f3, 2.2);

            const dryLen = Math.floor(sampleRate * 0.024); // 24ms dry strike
            const dry = new Float32Array(dryLen);

            for (let i = 0; i < dryLen; i++) {
                const t = i / sampleRate;
                let impulse = 0;
                if (t < 0.001) {
                    impulse = (Math.random() * 2 - 1) * Math.exp(-t * 2600);
                }
                dry[i] = bp1.process(impulse) * 0.58 + bp2.process(impulse) * 0.30 + bp3.process(impulse) * 0.16;
            }

            // Distant Room Acoustics Simulation (Jam Dinding di Dinding Seberang Ruangan):
            // 1. Direct path with subtle interaural time difference (ITD ~0.3ms to place on room wall)
            const directDelayL = 0;
            const directDelayR = Math.floor(0.00028 * sampleRate);

            for (let i = 0; i < dryLen; i++) {
                if (i + directDelayL < length) outL[i + directDelayL] += dry[i] * 0.82;
                if (i + directDelayR < length) outR[i + directDelayR] += dry[i] * 0.68;
            }

            // 2. Early room reflections off adjacent walls, ceiling, and floor (creates realistic distance & depth)
            const reflections = [
                { delayL: 0.008, delayR: 0.009, gain: 0.44, lp: 0.65 }, // Wall reflection directly behind clock
                { delayL: 0.017, delayR: 0.016, gain: 0.28, lp: 0.50 }, // Ceiling bounce
                { delayL: 0.027, delayR: 0.029, gain: 0.18, lp: 0.38 }, // Floor bounce
                { delayL: 0.042, delayR: 0.039, gain: 0.10, lp: 0.26 }  // Opposite wall diffuse return
            ];

            for (const ref of reflections) {
                const startL = Math.floor(ref.delayL * sampleRate);
                const startR = Math.floor(ref.delayR * sampleRate);
                let lpL = 0, lpR = 0;

                for (let i = 0; i < dryLen; i++) {
                    lpL += (dry[i] - lpL) * ref.lp;
                    lpR += (dry[i] - lpR) * ref.lp;
                    if (startL + i < length) outL[startL + i] += lpL * ref.gain;
                    if (startR + i < length) outR[startR + i] += lpR * ref.gain;
                }
            }

            // Normalize peak to 0.85 so it stays distinctly loud & clear without clipping
            let maxAmp = 0;
            for (let i = 0; i < length; i++) {
                if (Math.abs(outL[i]) > maxAmp) maxAmp = Math.abs(outL[i]);
                if (Math.abs(outR[i]) > maxAmp) maxAmp = Math.abs(outR[i]);
            }
            if (maxAmp > 0) {
                const scale = 0.85 / maxAmp;
                for (let i = 0; i < length; i++) {
                    outL[i] *= scale;
                    outR[i] *= scale;
                }
            }

            if (isTock) this._tockBuffer = buffer;
            else this._tickBuffer = buffer;
            return buffer;
        }

        playWallClockTick(time, isTock = false) {
            if (!this.tracks.clock || !this.isPlaying || this.isMasterMuted) return;

            const source = this.ctx.createBufferSource();
            source.buffer = this.getClockTickBuffer(isTock);

            const gain = this.ctx.createGain();
            const vel = 0.92 + (Math.random() * 0.04 - 0.02);
            gain.gain.setValueAtTime(vel, time);

            source.connect(gain);
            gain.connect(this.tracks.clock.gainNode);

            source.start(time);
        }

        startClockTrack() {
            if (this.tracks.clock) return;
            const gainNode = this.ctx.createGain();
            const baseVolume = (this.settings.tracks.clock ? this.settings.tracks.clock.volume : 50) / 100;
            gainNode.gain.setValueAtTime(baseVolume, this.ctx.currentTime);
            gainNode.connect(this.masterGain);

            this.tracks.clock = { gainNode, active: true, timer: null, lastScheduledSec: -1 };

            // Real-time synchronization to wall clock seconds (Date.now())
            const scheduleClockTicks = () => {
                if (!this.tracks.clock || !this.isPlaying) return;

                const nowMs = Date.now();
                const currentSec = Math.floor(nowMs / 1000);
                const nextSec = currentSec + 1;
                const msUntilNextSec = (nextSec * 1000) - nowMs;

                // Lookahead scheduling: if upcoming second boundary is within 500ms and not yet scheduled
                if (this.tracks.clock.lastScheduledSec < nextSec && msUntilNextSec <= 500) {
                    this.tracks.clock.lastScheduledSec = nextSec;

                    const audioDelay = msUntilNextSec / 1000;
                    const targetAudioTime = Math.max(this.ctx.currentTime, this.ctx.currentTime + audioDelay);
                    const isTock = (nextSec % 2 !== 0);

                    this.playWallClockTick(targetAudioTime, isTock);
                } else if (this.tracks.clock.lastScheduledSec < currentSec) {
                    // When first started, if we happen to be within the first 120ms of the current second, play immediately
                    const msIntoCurrentSec = nowMs % 1000;
                    if (msIntoCurrentSec < 120) {
                        this.tracks.clock.lastScheduledSec = currentSec;
                        const isTock = (currentSec % 2 !== 0);
                        this.playWallClockTick(this.ctx.currentTime, isTock);
                    }
                }

                this.tracks.clock.timer = setTimeout(scheduleClockTicks, 40);
            };

            scheduleClockTicks();
        }

        stopClockTrack() {
            if (this.tracks.clock) {
                if (this.tracks.clock.timer) clearTimeout(this.tracks.clock.timer);
                delete this.tracks.clock;
            }
        }

        startOceanTrack() {
            if (this.tracks.ocean) return;
            const gainNode = this.ctx.createGain();
            gainNode.gain.setValueAtTime(this.settings.tracks.ocean.volume / 100, this.ctx.currentTime);
            gainNode.connect(this.masterGain);

            const noise = this.ctx.createBufferSource();
            noise.buffer = this.createNoiseBuffer('brown', 6);
            noise.loop = true;

            const filter = this.ctx.createBiquadFilter();
            filter.type = 'lowpass';
            filter.frequency.setValueAtTime(250, this.ctx.currentTime);

            const lfo = this.ctx.createOscillator();
            lfo.frequency.setValueAtTime(0.09, this.ctx.currentTime);
            const lfoGain = this.ctx.createGain();
            lfoGain.gain.setValueAtTime(350, this.ctx.currentTime);
            lfo.connect(lfoGain);
            lfoGain.connect(filter.frequency);

            noise.connect(filter);
            filter.connect(gainNode);

            noise.start();
            lfo.start();

            this.tracks.ocean = { gainNode, nodes: [noise, lfo], active: true };
        }
        1
        stopOceanTrack() {
            if (this.tracks.ocean) {
                if (this.tracks.ocean.nodes) {
                    this.tracks.ocean.nodes.forEach(n => {
                        try { n.stop(); } catch (e) { }
                    });
                }
                delete this.tracks.ocean;
            }
        }

        startAlphaTrack() {
            if (this.tracks.alpha) return;
            const gainNode = this.ctx.createGain();
            gainNode.gain.setValueAtTime(this.settings.tracks.alpha.volume / 100, this.ctx.currentTime);
            gainNode.connect(this.masterGain);

            const merger = this.ctx.createChannelMerger(2);

            const oscLeft = this.ctx.createOscillator();
            oscLeft.type = 'sine';
            oscLeft.frequency.setValueAtTime(432, this.ctx.currentTime);

            const oscRight = this.ctx.createOscillator();
            oscRight.type = 'sine';
            oscRight.frequency.setValueAtTime(442, this.ctx.currentTime);

            oscLeft.connect(merger, 0, 0);
            oscRight.connect(merger, 0, 1);
            merger.connect(gainNode);

            oscLeft.start();
            oscRight.start();

            this.tracks.alpha = { gainNode, nodes: [oscLeft, oscRight], active: true };
        }

        stopAlphaTrack() {
            if (this.tracks.alpha) {
                if (this.tracks.alpha.nodes) {
                    this.tracks.alpha.nodes.forEach(n => {
                        try { n.stop(); } catch (e) { }
                    });
                }
                delete this.tracks.alpha;
            }
        }

        startAllActiveTracks() {
            this.initContext();
            this.isPlaying = true;
            if (this.settings.tracks.typing && this.settings.tracks.typing.active) this.startTypingTrack();
            if (this.settings.tracks.fire && this.settings.tracks.fire.active) this.startFireTrack();
            if (this.settings.tracks.cricket && this.settings.tracks.cricket.active) this.startCricketTrack();
            if (this.settings.tracks.rain_drizzle && this.settings.tracks.rain_drizzle.active) this.startRainDrizzleTrack();
            if (this.settings.tracks.rain_heavy && this.settings.tracks.rain_heavy.active) this.startRainHeavyTrack();
            if (this.settings.tracks.night && this.settings.tracks.night.active) this.startNightTrack();
            if (this.settings.tracks.clock && this.settings.tracks.clock.active) this.startClockTrack();
            if (this.settings.tracks.ocean && this.settings.tracks.ocean.active) this.startOceanTrack();
            if (this.settings.tracks.alpha && this.settings.tracks.alpha.active) this.startAlphaTrack();
        }

        stopAllTracks() {
            this.isPlaying = false;
            this.stopTypingTrack();
            this.stopFireTrack();
            this.stopCricketTrack();
            this.stopRainDrizzleTrack();
            this.stopRainHeavyTrack();
            this.stopNightTrack();
            this.stopClockTrack();
            this.stopOceanTrack();
            this.stopAlphaTrack();
        }

        setTrackVolume(soundKey, vol) {
            if (!this.settings.tracks[soundKey]) this.settings.tracks[soundKey] = { active: false, volume: 50 };
            this.settings.tracks[soundKey].volume = vol;
            if (this.tracks[soundKey] && this.tracks[soundKey].gainNode && this.ctx) {
                this.tracks[soundKey].gainNode.gain.setValueAtTime(vol / 100, this.ctx.currentTime);
            }
            this.save();
        }

        setTrackActive(soundKey, active) {
            if (!this.settings.tracks[soundKey]) this.settings.tracks[soundKey] = { active: false, volume: 50 };
            this.settings.tracks[soundKey].active = active;
            if (active) {
                if (this.isPlaying) {
                    if (soundKey === 'typing') this.startTypingTrack();
                    else if (soundKey === 'fire') this.startFireTrack();
                    else if (soundKey === 'cricket') this.startCricketTrack();
                    else if (soundKey === 'rain_drizzle') this.startRainDrizzleTrack();
                    else if (soundKey === 'rain_heavy') this.startRainHeavyTrack();
                    else if (soundKey === 'night') this.startNightTrack();
                    else if (soundKey === 'clock') this.startClockTrack();
                    else if (soundKey === 'ocean') this.startOceanTrack();
                    else if (soundKey === 'alpha') this.startAlphaTrack();
                }
            } else {
                if (soundKey === 'typing') this.stopTypingTrack();
                else if (soundKey === 'fire') this.stopFireTrack();
                else if (soundKey === 'cricket') this.stopCricketTrack();
                else if (soundKey === 'rain_drizzle') this.stopRainDrizzleTrack();
                else if (soundKey === 'rain_heavy') this.stopRainHeavyTrack();
                else if (soundKey === 'night') this.stopNightTrack();
                else if (soundKey === 'clock') this.stopClockTrack();
                else if (soundKey === 'ocean') this.stopOceanTrack();
                else if (soundKey === 'alpha') this.stopAlphaTrack();
            }
            this.save();
        }

        setMasterVolume(vol) {
            this.settings.masterVolume = vol;
            if (this.masterGain && this.ctx) {
                this.masterGain.gain.setValueAtTime((vol / 100) * (this.isMasterMuted ? 0 : 1), this.ctx.currentTime);
            }
            this.save();
        }

        toggleMasterMute() {
            this.isMasterMuted = !this.isMasterMuted;
            if (this.masterGain && this.ctx) {
                const vol = this.isMasterMuted ? 0 : (this.settings.masterVolume / 100);
                this.masterGain.gain.setValueAtTime(vol, this.ctx.currentTime);
            }
            return this.isMasterMuted;
        }

        applyPreset(presetName) {
            const presets = {
                night_coder: { typing: { active: true, volume: 65 }, fire: { active: false, volume: 55 }, cricket: { active: true, volume: 50 }, rain_drizzle: { active: false, volume: 45 }, rain_heavy: { active: false, volume: 45 }, night: { active: true, volume: 35 }, clock: { active: false, volume: 40 }, ocean: { active: false, volume: 45 }, alpha: { active: false, volume: 30 } },
                campfire_cozy: { typing: { active: false, volume: 50 }, fire: { active: true, volume: 70 }, cricket: { active: true, volume: 45 }, rain_drizzle: { active: false, volume: 40 }, rain_heavy: { active: false, volume: 40 }, night: { active: true, volume: 30 }, clock: { active: false, volume: 35 }, ocean: { active: false, volume: 45 }, alpha: { active: false, volume: 30 } },
                rainy_cafe: { typing: { active: false, volume: 60 }, fire: { active: false, volume: 40 }, cricket: { active: false, volume: 40 }, rain_drizzle: { active: true, volume: 65 }, rain_heavy: { active: false, volume: 40 }, night: { active: false, volume: 40 }, clock: { active: true, volume: 50 }, ocean: { active: false, volume: 45 }, alpha: { active: false, volume: 30 } },
                office_flow: { typing: { active: true, volume: 75 }, fire: { active: false, volume: 40 }, cricket: { active: false, volume: 40 }, rain_drizzle: { active: false, volume: 40 }, rain_heavy: { active: false, volume: 40 }, night: { active: false, volume: 40 }, clock: { active: true, volume: 45 }, ocean: { active: false, volume: 45 }, alpha: { active: true, volume: 35 } },
                zen_mind: { typing: { active: false, volume: 50 }, fire: { active: false, volume: 40 }, cricket: { active: false, volume: 40 }, rain_drizzle: { active: true, volume: 35 }, rain_heavy: { active: false, volume: 40 }, night: { active: false, volume: 40 }, clock: { active: false, volume: 30 }, ocean: { active: true, volume: 40 }, alpha: { active: true, volume: 55 } },
                ocean_night: { typing: { active: false, volume: 50 }, fire: { active: false, volume: 40 }, cricket: { active: true, volume: 35 }, rain_drizzle: { active: false, volume: 40 }, rain_heavy: { active: false, volume: 40 }, night: { active: true, volume: 45 }, clock: { active: false, volume: 30 }, ocean: { active: true, volume: 65 }, alpha: { active: false, volume: 30 } },
                heavy_downpour: { typing: { active: false, volume: 50 }, fire: { active: false, volume: 40 }, cricket: { active: false, volume: 40 }, rain_drizzle: { active: false, volume: 40 }, rain_heavy: { active: true, volume: 75 }, night: { active: true, volume: 40 }, clock: { active: false, volume: 30 }, ocean: { active: false, volume: 40 }, alpha: { active: false, volume: 30 } },
                focus_clock: { typing: { active: false, volume: 50 }, fire: { active: false, volume: 40 }, cricket: { active: false, volume: 40 }, rain_drizzle: { active: false, volume: 40 }, rain_heavy: { active: false, volume: 40 }, night: { active: false, volume: 40 }, clock: { active: true, volume: 60 }, ocean: { active: false, volume: 40 }, alpha: { active: true, volume: 30 } }
            };

            const p = presets[presetName];
            if (!p) return;

            this.settings.activePreset = presetName;
            for (const key in p) {
                this.setTrackVolume(key, p[key].volume);
                this.setTrackActive(key, p[key].active);
            }
            this.save();
        }

        save() {
            userData.ambientSettings = this.settings;
            saveUserData();
        }
    }

    const ambientEngine = new AmbientAudioEngine();
    window.ambientEngine = ambientEngine;

    // Ambient Mixer Modal UI Binding
    const ambientModal = document.getElementById('ambient-mixer-modal');
    const btnCloseAmbientModal = document.getElementById('btn-close-ambient-modal');
    const btnAmbientMasterToggle = document.getElementById('btn-ambient-master-toggle');
    const ambientMasterStatusText = document.getElementById('ambient-master-status-text');
    const ambientMasterVolume = document.getElementById('ambient-master-volume');
    const ambientMasterVolLabel = document.getElementById('ambient-master-vol-label');
    const btnAmbientMuteAll = document.getElementById('btn-ambient-mute-all');
    const chkAmbientAutoplay = document.getElementById('chk-ambient-autoplay');

    // Open Ambient Modal triggers
    document.querySelectorAll('#btn-open-ambient-mixer, .btn-open-ambient-trigger').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e?.stopPropagation();
            document.querySelectorAll('.modal-overlay.active').forEach(m => m.classList.remove('active'));
            syncAmbientUI();
            if (ambientModal) ambientModal.classList.add('active');
            ambientEngine.initContext();
            if (!ambientEngine.isPlaying) {
                ambientEngine.startAllActiveTracks();
                syncAmbientUI();
            }
        });
    });

    if (btnCloseAmbientModal && ambientModal) {
        btnCloseAmbientModal.addEventListener('click', () => {
            ambientModal.classList.remove('active');
        });
    }

    function syncAmbientUI() {
        if (!ambientEngine) return;
        const s = ambientEngine.settings;
        if (ambientMasterVolume) ambientMasterVolume.value = s.masterVolume || 80;
        if (ambientMasterVolLabel) ambientMasterVolLabel.textContent = `${s.masterVolume || 80}%`;
        if (chkAmbientAutoplay) chkAmbientAutoplay.checked = (s.autoPlay !== false);

        if (btnAmbientMasterToggle) {
            if (ambientEngine.isMasterMuted || !ambientEngine.isPlaying) {
                btnAmbientMasterToggle.className = 'ambient-master-btn muted';
                if (ambientMasterStatusText) ambientMasterStatusText.textContent = 'Audio Muted';
            } else {
                btnAmbientMasterToggle.className = 'ambient-master-btn active';
                if (ambientMasterStatusText) ambientMasterStatusText.textContent = 'Audio Active';
            }
        }

        // Preset Chips
        document.querySelectorAll('.ambient-preset-chip').forEach(chip => {
            const preset = chip.getAttribute('data-preset');
            if (preset === s.activePreset) {
                chip.classList.add('active');
            } else {
                chip.classList.remove('active');
            }
        });

        // Tracks Cards & Sliders
        document.querySelectorAll('.ambient-track-card').forEach(card => {
            const soundKey = card.getAttribute('data-sound');
            const trackCfg = s.tracks[soundKey] || { active: false, volume: 50 };
            const toggle = card.querySelector('.track-toggle');
            const slider = card.querySelector('.track-volume-slider');
            const volText = card.querySelector('.track-vol-text');

            if (toggle) toggle.checked = !!trackCfg.active;
            if (slider) slider.value = trackCfg.volume || 50;
            if (volText) volText.textContent = `${trackCfg.volume || 50}%`;

            if (trackCfg.active && ambientEngine.isPlaying && !ambientEngine.isMasterMuted) {
                card.classList.add('active');
            } else {
                card.classList.remove('active');
            }
        });
    }

    // Preset Chips Click
    document.querySelectorAll('.ambient-preset-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            const preset = chip.getAttribute('data-preset');
            ambientEngine.applyPreset(preset);
            syncAmbientUI();
        });
    });

    // Track Toggles & Volume Sliders
    document.querySelectorAll('.track-toggle').forEach(toggle => {
        toggle.addEventListener('change', (e) => {
            const soundKey = e.target.getAttribute('data-sound');
            ambientEngine.setTrackActive(soundKey, e.target.checked);
            syncAmbientUI();
        });
    });

    document.querySelectorAll('.track-volume-slider').forEach(slider => {
        slider.addEventListener('input', (e) => {
            const soundKey = e.target.getAttribute('data-sound');
            const vol = parseInt(e.target.value, 10) || 0;
            ambientEngine.setTrackVolume(soundKey, vol);
            const volText = document.querySelector(`.track-vol-text[data-sound="${soundKey}"]`);
            if (volText) volText.textContent = `${vol}%`;
        });
    });

    // Master Controls
    if (btnAmbientMasterToggle) {
        btnAmbientMasterToggle.addEventListener('click', () => {
            if (!ambientEngine.isPlaying) {
                ambientEngine.startAllActiveTracks();
            } else {
                ambientEngine.toggleMasterMute();
            }
            syncAmbientUI();
        });
    }

    if (btnAmbientMuteAll) {
        btnAmbientMuteAll.addEventListener('click', () => {
            ambientEngine.toggleMasterMute();
            syncAmbientUI();
        });
    }

    if (ambientMasterVolume) {
        ambientMasterVolume.addEventListener('input', (e) => {
            const vol = parseInt(e.target.value, 10) || 0;
            ambientEngine.setMasterVolume(vol);
            if (ambientMasterVolLabel) ambientMasterVolLabel.textContent = `${vol}%`;
        });
    }

    if (chkAmbientAutoplay) {
        chkAmbientAutoplay.addEventListener('change', (e) => {
            ambientEngine.settings.autoPlay = e.target.checked;
            ambientEngine.save();
        });
    }

    // Hook ambient audio playback into Focus Session events
    const originalStartSession = btnStartFocus.onclick;
    btnStartFocus.addEventListener('click', () => {
        if (ambientEngine.settings.autoPlay !== false) {
            ambientEngine.startAllActiveTracks();
        }
    });

    btnStop.addEventListener('click', () => {
        ambientEngine.stopAllTracks();
        syncAmbientUI();
    });

    btnPause.addEventListener('click', () => {
        if (isPaused) {
            if (ambientEngine.settings.autoPlay !== false) {
                ambientEngine.startAllActiveTracks();
            }
        } else {
            ambientEngine.stopAllTracks();
        }
        syncAmbientUI();
    });

    let _lastPrayerDateKey = '';
    function updatePrayerClock() {
        if (document.hidden) return;
        if (typeof checkAndHandleMidnightRollover === 'function') {
            checkAndHandleMidnightRollover();
        }

        if (!prayerClockDisplay) return;
        const now = new Date();
        if (setupView && setupView.classList.contains('active')) {
            prayerClockDisplay.textContent = now.toLocaleTimeString();
            const dateKey = `${now.getFullYear()}-${now.getMonth()}-${now.getDate()}`;
            if (prayerDateDisplay && _lastPrayerDateKey !== dateKey) {
                _lastPrayerDateKey = dateKey;
                prayerDateDisplay.textContent = now.toLocaleDateString('id-ID', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });
            }
            updateSetupPrediction();
        }
    }
    setInterval(updatePrayerClock, 1000);

    // Auto-poll running browser apps every 2.5 seconds to sync YouTube Music track titles (throttled when hidden or in PIP mode)
    setInterval(() => {
        if (document.hidden || document.body.classList.contains('pip-mode')) return;
        sendToCpp({ action: 'getRunningApps' });
    }, 2500);
});

