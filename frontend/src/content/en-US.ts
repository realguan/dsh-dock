// English locale dictionary for DSH Dock (4.13 i18n).
import type { AppCopy } from "./zh-CN"

export const enUS: AppCopy = {
  boot: {
    progressLabel: "Starting up",
    steps: [
      { no: "01", name: "Environment Check", hint: "Making sure your computer is ready to run DSH" },
      { no: "02", name: "Prepare Engine", hint: "Getting DSH components ready (first run downloads them)" },
      { no: "03", name: "Launch Workbench", hint: "Starting the DSH service" },
      { no: "04", name: "Awaiting Ready", hint: "Almost there — first launch can take a moment" },
      { no: "05", name: "Enter Workbench", hint: "Opening the workbench for you" },
    ],
    headlines: [
      "Checking Runtime Engine",
      "Preparing Runtime Environment",
      "Launching Workbench",
      "Waiting for Service Ready",
      "Entering Workbench Soon",
    ],
    stRunning: "Running",
    copyDetail: "Copy details",
    copied: "Copied",
    progressAria: "Startup progress: {done} of {total}",
    dlPackages: "{done} / {total} packages",
    // Download-card static copy (2026-09-18): was hardcoded Chinese inside
    // components/boot/DownloadProgress.tsx, visible to en users.
    dlKindNode: "Node.js runtime",
    dlKindDsh: "DSH engine",
    dlEngineBadge: "Engine bootstrap",
    dlTransferred: (bytes: string) => `Transferred ${bytes}`,
    dlMirrorNote: "Official mirrors · integrity verified",
    dlSelfContainedNote: "Self-contained engine · offline after first run",
    wslOpen: "Open in WSL",
    wslOpenTip: "Run DSH inside WSL2 distro (requires Windows + WSL2)",
    wslFailed: "Failed to switch to WSL",
    localOpen: "Open Locally",
    localOpenTip: "Switch back to local mode",
    localFailed: "Failed to switch to local",
    controlCenter: "Control Center",
    controlCenterTip: "Open Control Center (manage profiles, plugins, credentials, diagnostics)",
    launchingTitle: "Launching Workbench...",
    launchingSub: "Connecting to local service and runtime, entering workbench soon",
    viewTimeline: "View startup details",
    hideTimeline: "Hide details",
  },
  handoff: {
    titleStart: (name: string) => `Starting "${name}"`,
    titleRestart: (name: string) => `Restarting "${name}"`,
    titleSwitch: (name: string) => `Switching to "${name}"`,
    stageStopping: "Stopping session",
    stageBooting: "Starting session",
    stageWaiting: "Waiting for ready",
    stageEntering: "Entering workbench",
    phaseStopping: "Stopping the current session…",
    phaseBooting: "Preparing and starting the new session…",
    phaseWaiting: "Service started, waiting until it is ready…",
    phaseEntering: "Ready — opening the workbench…",
    phaseReady: "Workbench is ready",
    phaseFailed: "Launch interrupted — see the error card in the main window",
    elapsedTitle: "Elapsed time for this operation (continuous across windows)",
    focusWorkbench: "View progress",
    enterWorkbench: "Open workbench",
    viewFailure: "View error details",
    dismissFailure: "Dismiss this failure notice",
  },
  error: {
    fallbackTitle: "Launch Failed",
    // Action-failure detail (2026-09-11, task-27): was composed in the component as
    // `${t.error.actionFailed}：${msg}` — the separator was a FULL-WIDTH CJK colon,
    // so en users saw "Action Failed：…". Now a composed key (same shape as
    // `market.installFailed`): zh uses the full-width colon with no space,
    // en uses ASCII colon + space. The standalone `actionFailed` label lost its
    // only consumer and was removed with it.
    actionFailedDetail: (msg: string) => `Action Failed: ${msg}`,
    // Clipboard write failure (2026-09-08, batch 0b): never fake a "copied" state
    copyFailed: "Copy failed — please select the text and copy manually",
    actions: {
      retry: "Retry",
      upgrade: "Upgrade DSH & Retry",
      upgrade_only: "Upgrade in Background",
      reselect: "Reselect",
      // v1.2.0 D1 (task-52): the way out when the local Windows mode fails on
      // symlink privilege. Contract: boot_failure.rs:135
      // `vec!["boot_in_wsl", "retry"]`. Without this key the label falls back to
      // the raw English id `boot_in_wsl` (`actionLabel`'s `?? id`).
      boot_in_wsl: "Switch to WSL mode",
      // 2026-09-16: in-place way out when a plugin's mount row breaks the plugin tree
      // (remove that row only, then restart). Contract: boot_failure.rs::with_quarantine
      // -> `advancedActions` (never on the first screen).
      quarantine_plugin_row: "Remove only that row & restart",
      // ADR-0025 safe mode. 2026-09-16 (maintainer feedback): the first screen keeps
      // exactly one action -- the one that gets you back into the app; the reset
      // variant moved into "Other ways out".
      safe_mode: "Turn off all third-party plugins & start",
      safe_mode_reset: "Back up & empty the plugin config, then start",
    } as Record<string, string>,
    // Action -> **what it does** (shown next to each button on the first screen).
    // Mirrors `ErrorCard.tsx`'s ACTION_IPC set: a new action must add copy on both sides.
    impacts: {
      safe_mode: "Disables every third-party plugin in the profile config (backed up first); re-enable what you need from the Plugins page",
      safe_mode_reset: "For an unparsable plugin config: backs it up as .bak-<timestamp>, then empties the file",
      quarantine_plugin_row: "Deletes that row from cordis.patch.yml (auto-backed up first)",
      retry: "Runs the exact same startup flow again",
      upgrade: "Upgrades DSH first, then retries; touches only the pnpm/npm global, not your data",
      upgrade_only: "Upgrades DSH only, without restarting the current session",
      boot_in_wsl: "Starts inside WSL instead, bypassing local-mode privilege limits",
      reselect: "Back to profile selection",
    } as Record<string, string>,
    // Heading of the collapsed "other ways out" block (2026-09-16).
    advancedLabel: "Other ways out (rarely needed)",
    safeModeResetTitle: "Back up & empty the plugin config, then start?",
    safeModeResetNote: "Use this when the patch file is unparsable and rows cannot even be listed.",
    safeModeResetPointBackup:
      "Your cordis.patch.yml is backed up as .bak-<timestamp> first, and can be restored by hand",
    safeModeResetPointScope:
      "Every plugin mount row of this profile stops taking effect: the workbench starts with shipped capabilities only",
    safeModeResetConfirm: "Back up & continue",
    // Failure-detail suffix (2026-09-11, task-25): was an inline literal in the
    // component. ASCII parens + a leading space (zh uses full-width parens, which
    // need no space); composed as `${msg}${reselectHint}`.
    reselectHint: " (you can go back and reselect)",
    // Error-card collapse/expand (v1.2.0 report 2.1): the card could only be read,
    // not dismissed, so a stale card kept fighting the wizard. Collapsing folds the
    // body only — the header (icon + title + index) stays visible, so the error is
    // never silently hidden.
    collapseDetail: "Collapse",
    expandDetail: "Expand",
    // Error-card static labels (2026-09-08, ADR-0012)
    diagHeader: "DIAG Console",
    cardHeader: "Launch Interrupted",
    suggestionLabel: "Suggested fix:",
    // Diagnostic-log details block (2026-09-11, task-27): were inline literals in
    // ErrorCard (Chinese, visible to en users). `copyLog` differs from
    // `boot.copyDetail` (BootStep's "Copy details" — step telemetry, not the raw
    // log), so it is its own key. The copied state reuses `boot.copied`.
    copyLog: "Copy log",
    rawLogSummary: (n: number) => `Raw diagnostic log · last ${n} lines`,
    // Per-kind copy (ADR-0012): looked up by failure.kind, falling back to the
    // backend-provided title/suggestion when the kind is unknown.
    kinds: {
      credentials_mismatch: {
        title: "Your DSH host does not match your credential format",
        suggestion:
          "This usually means DSH is too old: upgrading to the latest official release fixes it (the upgrade only touches the global pnpm/npm, never your data).",
      },
      incompatible_options: {
        title: "DSH host options are incompatible",
        suggestion: "Upgrade your DSH to a version that supports the current terminal behavior.",
      },
      network_unavailable: {
        title: "Network unavailable",
        suggestion: "Live downloads need a network connection; check your network and retry.",
      },
      // v1.2.0 D1 (task-52): aligned with Rust `boot_failure.rs::title/suggestion`.
      // Design point (D1 §4): **rule out the wrong direction first** — users
      // naturally suspect the network, but the real cause is missing Windows
      // symlink privilege; not saying so wastes their time.
      symlink_privilege_required: {
        title: "Insufficient system permission: cannot create symbolic links",
        suggestion:
          "This is not a network problem: the engine needs to create a symbolic link, and the current Windows account lacks that privilege. The easiest fix is to switch to the \"WSL\" environment (pick it on the launch screen, or set it as the default in Control Center) — an already-installed WSL distro does not need that privilege. If you must stay in local mode: run this app as administrator, or enable Developer Mode in Settings → System → For developers, then retry.",
      },
      unknown: {
        title: "DSH workbench failed to start",
        suggestion: "See the log for details; retry, and report it if it keeps happening.",
      },
    } as Record<string, { title: string; suggestion: string }>,
  },
  mode: {
    title: "Select Runtime Environment",
    subline: "Windows supports running natively or inside a WSL2 distro. Please choose your default environment for first launch.",
    local: "Local Mode (Native Windows)",
    localDesc: "Run directly in Windows. Node & DSH bootstrapped automatically on first launch; instant startup & low memory.",
    localBadge: "Recommended · Native Speed",
    wsl: "WSL Mode (WSL2 Linux)",
    wslDesc: "Run DSH inside WSL2 Linux (requires Windows + WSL2; engine automatically bootstrapped inside guest).",
    wslBadge: "Linux Isolated Environment",
    selectedNotice: "Selected",
    setDefault: "Set as default launch mode (used automatically next time)",
    changeAnytime: "Can be changed anytime in Settings or Tray menu",
    next: "Start",
    starting: "Initializing environment…",
    failed: "Failed to initialize environment",
  },
  selector: {
    title: "Select Workbench",
    subtitle: "Multiple webUi workbenches available. Choose one to start.",
    headline: "Which workbench would you like to enter?",
    subline: "Select a workspace to start · Set as default to enter directly next time",
    pickHint: "Official Web workbench works out of the box; others are customized webUi workspaces.",
    launchingPrefix: "Launching \"",
    launchingSuffix: "\"",
    preparingTitle: "Preparing Engine Runtime",
    preparingSub: "Bootstrapping Node & DSH runtime for first launch · One-time setup",
    problemHeadline: "Startup Encountered an Issue",
    // 2026-09-11 (task-27): the `tag` field and the `customTag` key were removed —
    // after the task-23 decoupling they had no consumers left (the only remnant was
    // dead component code copying a dictionary value into the fallback object).
    items: {
      web: { title: "Official Web Workbench", desc: "Official Web interface maintained by DSH" },
    } as Record<string, { title: string; desc: string }>,
    customDesc: "Custom assembled webUi workbench",
    chipDshNew: "Update Available",
    chipDshOk: "Up to Date",
    chipDetecting: "Detecting",
    chipCheckFailed: "Check Failed",
    chipClientNew: "App Update Available",
    chipClientUpdating: "Updating App",
    chipClientUpdatingRun: "Updating App…",
    // Workbench Launchpad
    rememberChoice: "Remember my choice and enter this workbench directly next time",
    rememberChoiceSub: "Skip this launcher on startup and open this workbench directly. Can be changed anytime.",
    setDefaultAction: "Set as default",
    defaultSetSuccess: "Set as default workbench",
    currentDefaultNotice: "Current default",
    recommendedBadge: "Recommended",
    quickKeysHint: "Press keys 1-9 to quickly launch",
    createWorkbench: "Create New Workbench",
    createWorkbenchDesc: "Clone templates or customize workspaces in Control Center",
    manageWorkbenches: "Workbench Manager",
    engineReady: "Engine Ready",
    pluginsCount: "{count} plugins",
    defaultBadge: "DEFAULT",
    // Footer note for the official web workbench with no third-party deps
    // (2026-09-11, task-25): was an inline literal (Chinese, visible to en users).
    officialReadyToUse: "Official · ready to use",
    enterWorkbench: "Open Workbench",
    enterDefaultWorkbench: "Open Default Workbench",
    launching: "Starting…",
    // Loading state for the profile list (added 2026-09-18)
    profilesLoading: "Loading your workbenches…",
  },
  updateBanner: {
    dshTitle: "dsh v{latest} available (current v{current})",
    dshConsequence:
      "Upgrading brings security fixes and improvements; staying behind may block future upgrades when version requirements rise.",
    clientTitle: "App v{latest} available",
    clientConsequence: "Upgrade is recommended for fixes and improvements.",
    dismissTip: "Dismiss this version",
    entryHint: "Open the update center from the About entry in the menu / tray.",
  },
  about: {
    workbenchLabel: "Workbench instances",
    title: "About & Updates",
    tagline: "Update Center · Desktop Client & Runtime",
    clientLabel: "Desktop Client",
    phases: {
      idle: "Idle",
      checking: "Checking",
      available: "Available",
      upToDate: "Up to Date",
      downloading: "Downloading",
      installing: "Installing",
      relaunching: "Relaunching",
      done: "Done",
      failed: "Failed",
    },
    lines: {
      idle: "No check yet",
      checking: "Checking official release source",
      upToDate: "Already up to date",
      downloading: "Downloading new version",
      installing: "Installing update",
      relaunching: "Relaunching into new version",
      failedTitle: "Update Failed",
    },
    foundNew: "New Version Found",
    releaseNotes: "Release Notes",
    downloadBtn: "Download & Install",
    preparing: "Preparing installation…",
    checkBtn: "Check for Updates",
    updatedDone: "Updated to",
    restartNote: "Client updates are signed by official Releases and restart automatically after install.",
    envTitle: "Runtime Environment",
    dshLabel: "DSH",
    nodeLabel: "Node Runtime",
    notDetected: "Not Detected",
    detecting: "Detecting…",
    hasNew: "Update Available",
    latestIsNewest: "Up to Date",
    latestOfficial: "Official Latest",
    notYetLocal: "Not yet detected locally",
    checkFailedNet: "DSH Check Failed (Network Unreachable)",
    nodeFromEngine: "Shell engine · managed by the app",
    nodeFromSystem: "From your system",
    nodeManaged: "Managed by App · Automatically prepared",
    // Not-installed state (2026-09-10): report "not installed" truthfully and put
    // the *planned* version in parentheses — never render a plan as installed.
    nodeNotInstalled: "Not installed",
    nodeNotInstalledPlanned: (v: string) => `Not installed (planned ${v})`,
    dshUpgradeNote: "Upgrading DSH only updates global pnpm/npm packages and does not touch your data or configurations.",
    upgrading: "Upgrading…",
    upgradeFailed: "Upgrade Failed",
    upgradeRunning: "Installing globally via pnpm/npm, this may take several minutes…",
    btnCheck: "Check for Updates",
    btnUpgrade: "Upgrade",
    // Version picker (2026-09-09): "new version" only reflects the upgradable
    // line (stable/rc); preview builds are opt-in via the version list.
    btnUpgradeTo: "Upgrade to",
    installingVersion: "Installing",
    previewReleased: "Preview released:",
    versionListEntry: "All versions",
    noteUpgraded: "Upgraded to",
    noteAlreadyLatest: "Already latest",
    onPreview: "Preview build · stable line",
    versionListTitle: "DSH Versions",
    versionListHint: "Preview builds may be unstable — you can roll back anytime.",
    filterAll: "All",
    filterUpgradable: "Stable · RC",
    filterPreview: "Preview",
    channelStable: "Stable",
    channelRc: "RC",
    channelAlpha: "Preview",
    channelOther: "Preview",
    rowInstall: "Install",
    rowRollback: "Roll back",
    rowCurrent: "Current",
    listLoadFailed: "Failed to load versions",
    listRetry: "Retry",
    listEmpty: "No versions match this filter",
    cancelBtn: "Cancel",
    confirmAlphaTitle: "Install Preview Build",
    confirmAlphaNote:
      "is a preview build — it may be unstable, or its dependencies may not be fully published.",
    confirmAlphaPoints: [
      "Preview builds are for early adoption and feedback; not recommended for production use",
      "If something breaks, roll back to a stable version from the version list anytime",
    ],
    confirmRollbackTitle: "Roll Back to Older Version",
    confirmRollbackNote:
      "is older than the installed version — rolling back reinstalls the global dsh package.",
    confirmRollbackPoints: [
      "Rolling back only reinstalls the global dsh package; your data and configuration are not affected",
      "Sessions created on a newer version may not open on an older one",
    ],
    confirmInstallAnyway: "Install Anyway",
    confirmRollbackOk: "Roll Back",
    openInBrowser: "Open in Browser",
    workbenchNotReady: "Workbench not ready yet",
    liveSessionActive: "Workbench Session Active",
    copyDiagnostics: "Copy Diagnostics",
    diagnosticsCopied: "Diagnostics copied to clipboard",
    officialChannel: "Official Release Channel",
    // 2026-09-18 UI convergence: copy moved out of ClientUpdateCard /
    // DshVersionListDialog / About page (was hardcoded Chinese there).
    notesExpand: "Expand all",
    notesCollapse: "Collapse",
    fetchingRelease: "Fetching assets…",
    workbenchBridgeHint: "The local HTTP bridge is established automatically once DSH starts.",
    versionFilterLabel: "Version filter",
  },
  profiles: {
    aboutEntry: "About / Update",
    aboutEntryTip: "Open the About window (fallback when this machine has no tray / menu-bar entry)",
    moreActions: "More actions",
    listLabel: "Profile list",
    detailWorkspaceLabel: "Profile detail workspace",
    distributeTargetLabel: "Select target profile",
    distributeWithConfig: "Copy config lines too",
    mcpDisable: "Disable this MCP server",
    // Row-level toggle accessible names: with several rows on screen the name must say which.
    mcpEnableAria: (name: string) => `Enable MCP server ${name}`,
    mcpDisableAria: (name: string) => `Disable MCP server ${name}`,
    mcpEnvRemove: "Remove this environment variable",
    title: "Control Center",
    subtitle: "Workspaces management, plugin matrix, and system console",
    refresh: "Refresh",
    refreshing: "Refreshing…",
    refreshData: "Refresh Data",
    dataRefreshed: "Profile and plugin data refreshed",
    reloadingWorkbench: "Reloading workbench…",
    launchingProfile: "Starting service…",
    switchToDsh: "Return to DSH",
    switchToDshTip: "Quick switch back to DSH Workbench (Shortcut: ⌘ + , / Ctrl + , to toggle)",
    createBtn: "New Profile",
    loadFailed: "Failed to read profiles. Please verify DSH environment and retry.",
    retryLoad: "Retry",
    tagMaterialized: "Created",
    tagTemplate: "Template",
    tagNoUi: "Headless",
    templateHint: "Built-in template · Materialized automatically on first launch",
    defaultBadge: "Default",
    runningBadge: "Running",
    setDefault: "Set as Default",
    defaultIs: "Current Default",
    launch: "Launch",
    launchWorking: "Launching…",
    metaBundles: (n: number) => `${n} plugins`,
    metaDeps: (n: number) => (n > 0 ? `${n} dependencies` : "No extra dependencies"),
    metaSep: "·",
    detailTitle: (name: string) => `Details: "${name}"`,
    detailPackage: "Package Name",
    detailBundles: "Plugin Bundles (dsh.profile.bundles)",
    detailDeps: "Plugin List (dependencies)",
    detailPatch: "cordis.patch.yml Content",
    detailPatchNone: "No patch layer yet (generated by DSH on first run)",
    detailEmptyDeps: "This profile has no plugins yet",
    detailClose: "Close",
    pluginNotInstalled: "Not Installed",
    pluginAddBtn: "Add Plugin",
    pluginAddTitle: (name: string) => `Add Plugin to "${name}"`,
    pluginAddDesc: "Browse marketplace plugins, import from other profiles, or enter an npm package spec",
    pluginAddTabMarket: "Marketplace",
    pluginAddTabImport: "Import from Other",
    pluginAddTabCustom: "Custom Spec",
    pluginAddMarketSearch: "Search community plugins by name, desc, or tag...",
    pluginAddAlreadyInstalled: "Installed in this profile",
    pluginAddInstallAction: "Install",
    pluginAddInstalling: "Installing…",
    pluginAddCustomPlaceholder: "Enter npm package or spec, e.g. dsh-plugin-xxx or @scope/pkg@^1.0.0",
    pluginAddCustomSubmit: "Install to Profile",
    pluginAddCustomHint: "Supports public npm package names, version ranges, or prerelease tags",
    pluginAddEmptyMarket: "No matching community plugins found",
    pluginAddLoadMarketFailed: "Failed to load marketplace plugins. Please check network and retry",
    pluginAddMarketInstallDone: (name: string) => `Plugin "${name}" enqueued for installation`,
    pluginAddCategoryLabel: "Filter by category",
    pluginAddMarketLoading: "Loading the community plugin registry…",
    pluginAddMoreHidden: (n: number) =>
      `${n} more not listed — narrow down with search or a category`,
    pluginAddImportDone: "Import queue finished",
    pluginAddImportOk: "OK",
    pluginAddImportFrom: "from",
    pluginAddSpecLabel: "npm package name or version range",
    pluginDesktopRuntimeToggle: "Toggle runtime details",
    pluginInstallBusy: "Installing… (Downloading may take up to minutes)",
    pluginUninstall: "Uninstall",
    pluginUninstallConfirm: (pkg: string) => `Uninstall plugin "${pkg}"?`,
    pluginUninstallPoints: [
      "Runs dsh plugin remove, dropping it from this profile's dependencies",
      "The plugin's own config files are not deleted by this action",
      "If the profile is running, this takes effect after a restart; reinstalling is required to use it again",
    ],
    pluginUpdate: "Update",
    // Toggle copy (rewritten 2026-09-17 after a live question: "why do I see both
    // Disabled and Stopped?"). The old labels hardcoded "(Requires Restart)" — measured
    // false for web profiles: `dsh.profile.patchReload = live` makes dsh watch
    // cordis.patch.yml via chokidar; a clone measured fiber disposal 0.43s and
    // re-creation 0.44s after the file change. Labels now name the action only; whether it
    // took effect is reported by the row's runtime chip (Applying → Applied / Not applied).
    pluginDisable: "Disable",
    pluginEnable: "Enable",
    pluginToggleHint:
      "The switch writes this profile's config: some profiles apply it right away (web profiles: about half a second in our measurements), others only after a restart — the row's badge tells you which happened.",
    pluginDisabled: "Disabled",
    pluginDisabledHint:
      "Disabled in config (written to this profile's cordis.patch.yml) — dsh will not load this row.",
    // Runtime chip: describes only "what the running dsh looks like right now", kept
    // deliberately distinct from the config-side Disabled badge above.
    chip: {
      active: "Running",
      loading: "Loading",
      failed: "Failed",
      unloaded: "Not loaded",
      notApplied: "Not applied",
      applying: "Applying",
    },
    chipHint: {
      active: "This row is mounted and active inside the running dsh.",
      loading: "Still mounting (turns into \"Running\" once done).",
      failed: "Failed to mount: the plugin never came up (often an unresolvable package or a throwing apply).",
      unloaded:
        "Enabled in config, but this session has no instance of it — usually an import failure (package/dependency not resolvable).",
      notApplied:
        "The running dsh has not applied this change yet (this profile applies patches on startup): restart the profile to apply it.",
      applying: "Config written; waiting for the running dsh to apply this switch (about half a second in our measurements).",
    },
    toggleApplied: (pkg: string, on: boolean) =>
      `${on ? "Enabled" : "Disabled"} ${pkg} (applied)`,
    toggleRestart: (pkg: string, on: boolean) =>
      `${on ? "Enabled" : "Disabled"} ${pkg} (applies after restarting the profile)`,
    // Rows with no observable runtime entry (bundle-patch rows: their contributed rows carry
    // their own names, so the bundle name is never an entry): never promise a timing we cannot
    // observe — report only that the config was written.
    toggleDone: (pkg: string, on: boolean) =>
      `${on ? "Enabled" : "Disabled"} ${pkg} (config written)`,
    // Safe-mode banner (ADR-0026, fourth revision — rewritten from a PM standpoint):
    // ① shown only after the user actually entered via safe mode (journal-driven), dismissible,
    //    and stays dismissed for that entry; ② the copy states what happened and where to undo it
    //    one plugin at a time — no mention of backups (we offer no bulk restore) and no pointer to
    //    Experimental Capabilities (that lists curated capabilities only, while safe mode disables
    //    ALL third-party plugins).
    safeModeTitle: "Safe mode",
    safeModeBody: (n: number) =>
      `All third-party plugins were disabled so DSH could start (${n} currently off). ` +
      `Turn the ones you need back on from the Plugins page.`,
    safeModeDismiss: "Don't show again",
    safeModeDismissFailed: (msg: string) => `Could not save "don't show again": ${msg}`,
    pluginOpBusyRemove: "Uninstalling…",
    pluginOpBusyUpdate: "Updating…",
    checkUpdatesBtn: "Check Updates",
    updateCheckStarted: "Update check started — see menu badge & About",
    checkingBtn: "Checking…",
    updateChecked: (r: { checked: number; failed: number }) =>
      `Checked ${r.checked} plugins${r.failed ? ` · ${r.failed} failed` : ""}`,
    allUpToDate: "All up to date",
    updateHint: "Select version to update",
    pickVersionTitle: (pkg: string) => `Available versions for "${pkg}"`,
    versionLatest: "Latest",
    versionCurrent: "Current",
    versionsLoadFailed: "Failed to query versions",
    // 2026-09-19 审美批次 3：去掉与详情头徽标重复的 "Session active ·" 前缀，改纯计数图例。
    runtimeSummary: (s: { active: number; failed: number; loading: number; disabled: number }) =>
      `Running ${s.active}${s.failed ? ` · Failed ${s.failed}` : ""}${s.loading ? ` · Loading ${s.loading}` : ""}${s.disabled ? ` · Disabled ${s.disabled}` : ""}`,
    runtimeUnavailable: "Profile is not currently running; showing static manifest",
    runtimeFailedSummary: (n: number) =>
      `${n} runtime entries failed to load (see System Console logs)`,
    // Official desktop runtime note (2026-09-11): see zh-CN.ts for the ruling —
    // no hardcoded service count. Split into plain-text segments around the two
    // `<code>` identifiers, keeping surrounding punctuation and spaces.
    desktopRuntimeDescPrefix: "Belongs to the DeepSeek official desktop client (",
    desktopRuntimeDescMid: "), bundling locally preinstalled base services and core components (stored in the ",
    desktopRuntimeDescSuffix: " local package directory).",
    createTitle: "Create Profile",
    createNameLabel: "Profile Name",
    createNamePlaceholder: "e.g. my-workbench",
    createNameHelp: "Creating with same name retries installation (idempotent); existing complete names are rejected",
    createTemplateHint: (name: string) =>
      `"${name}" is a built-in template: will be initialized as official ${name === "web" ? "Web" : "Headless"} workbench`,
    createDefaultHint: "Will create Base + Web workbench (same structure as official web template, zero download), ready to launch or set as default",
    createSubmit: "Create",
    createBusy: "Creating… (Local setup, usually takes seconds)",
    createDoneReady: "Created: Base + Web workbench is ready to launch or set as default",
    createDonePending: "Created, but plugin installation is still pending — you can retry creation later",
    createDoneFailed: "Creation incomplete",
    createAgain: "Try Again",
    copyTitle: (name: string) => `Duplicate "${name}"`,
    renameTitle: (name: string) => `Rename "${name}"`,
    newNameLabel: "New Name",
    copyNote: "Duplication excludes node_modules (reinstalled by pnpm in new folder); all other files are copied as-is",
    renameNote: "node_modules will be cleaned up on rename and auto-repaired on next DSH launch",
    submitCopy: "Create Copy",
    submitRename: "Confirm Rename",
    actionRename: "Rename",
    actionDelete: "Delete",
    busyShort: "Processing…",
    renameSame: "New name is identical to current name",
    nameOccupied: "This name is already occupied",
    renameDone: (name: string) => `Renamed to "${name}"`,
    copyDone: (name: string) => `Created copy "${name}"`,
    warningsLabel: "Manual check recommended",
    opFailed: "Operation Failed",
    deleteTitle: (name: string) => `Delete "${name}"?`,
    deleteNote: "This action cannot be undone",
    deletePoint: [
      "Only this profile directory will be removed; global sessions and credentials remain untouched",
      "Ensure no other DSH processes are currently using this profile",
      "If this is a template name (web/headless), it will re-materialize on next launch",
    ] as readonly string[],
    deleteConfirm: (name: string) => `Confirm Delete ${name}`,
    deleteBusy: "Deleting…",
    deleteDoneCleared: "Deleted; was default profile, reverted to web",
    deleteDone: "Deleted",
    setDefaultDone: (name: string) => `Set "${name}" as default launch profile`,
    switchTitle: (name: string) => `Switch to "${name}"?`,
    switchNote: "Will stop current DSH instance and restart with target profile. Ongoing tasks will pause (session history preserved); default profile setting remains unchanged.",
    switchFrom: (active: string) => `Currently running: "${active}"`,
    switchConfirm: (name: string) => `Switch to "${name}"`,
    switchBusy: "Switching…",
    switchDone: (name: string) => `Switching to "${name}", see main window for progress`,
    restart: "Restart",
    restartTitle: (name: string) => `Restart "${name}"?`,
    restartNote: "Will stop current DSH instance and restart with same profile. Newly installed/uninstalled/disabled plugins will take effect.",
    restartConfirm: (name: string) => `Restart "${name}"`,
    restartBusy: "Restarting…",
    restartDone: (name: string) => `Restarting "${name}", see main window for progress`,
    viewProfiles: "Profiles",
    viewPluginHub: "Plugin Hub",
    viewPlugins: "Installed Plugins",
    viewMarket: "Marketplace",
    viewConsole: "Console",
    overviewSubtitle: "Third-party plugin distribution across all profiles",
    overviewSourceCount: (n: number) => `${n} profiles`,
    overviewEmpty: "No third-party plugins installed yet",
    overviewEmptyHint: "Plugins will be summarized here once installed in any profile",
    overviewNotInstalled: "Not Installed",
    importLoading: "Scanning other profiles…",
    importEmpty: "No third-party plugins in other profiles",
    importConfig: "With Config",
    importNoConfig: "No config to carry",
    importSelected: (n: number) => `Selected ${n}`,
    importStart: (n: number) => `Install ${n} items`,
    importRunning: (i: number, total: number) => `Installing ${i}/${total}`,
    importDone: (ok: number, fail: number) =>
      `Import complete: ${ok} succeeded${fail ? ` · ${fail} failed` : ""}`,
    searchPlaceholder: "Search profiles...",
    searchPluginsPlaceholder: "Search installed plugins...",
    searchAllPluginsPlaceholder: "Search plugins across all profiles...",
    tabPlugins: "Plugin List",
    // Experimental capabilities (ADR-0028: moved in from the plugin hub, tab next to Plugin List).
    tabCaps: "Experimental",
    // ── Plugin list, three kinds in one (2026-09-20, ADR-0028 batch 2: base layers /
    //    third-party / experimental in one list; the Base Bundles tab is retired) ──
    // Filter chips (same names as the three row markers). The "Built-in" filter includes
    // dsh-shipped experimental layers — they carry a Built-in marker too, so the three
    // counts can overlap (their sum can exceed the total); they are not a partition.
    listFilterBuiltin: "Built-in",
    listFilterThirdParty: "Community",
    listFilterExperimental: "Experimental",
    listFilterEmpty: "No plugins in this category yet",
    // Experimental rows are read-only in the plugin list (2026-09-21 ruling: the single
    // operation entry is the Experimental tab). Without saying so, users just see rows
    // with no switches and assume the UI is broken.
    experimentalReadOnlyNote:
      "Enabling, backend switching and removal for experimental plugins all happen under Experimental (this page only lists which packages are installed)",
    experimentalManageEntry: "Go to Experimental",
    listFilterShowAll: (n: number) => `Show all ${n} plugins`,
    // The three row markers (each plugin gets exactly one primary marker; dsh-shipped
    // experimental layers get an extra Built-in marker on top).
    tagBuiltin: "Built-in",
    tagBuiltinHint:
      "A layer shipped with the dsh installation: it cannot be uninstalled; switch it on the dsh plugins page",
    tagThirdParty: "Community",
    tagThirdPartyHint: "Community plugins (npm / GitHub): can be toggled, updated, and uninstalled",
    tagExperimental: "Experimental",
    tagExperimentalHint: (cap: string) =>
      `Belongs to the "${cap}" experimental capability: enabling, backend switching and removal all live under Experimental`,
    // 2026-09-21 third revision: the capability card is retired (the single operation
    // entry for experimental plugins is the "Experimental" tab), so these keys were
    // recycled: tagCapabilityBase(Hint) / tagBackend(Hint) / goToggle* /
    // capActiveBackend(Hint) / capNotToggleable(Hint) / capSwitchBackend(Hint) /
    // capVariantSwitched / capVariantSwitchFailed / capToggle* / capState*Hint / capUnlocks.
    capStateOn: "Enabled",
    capStateDisabled: "Disabled",
    capStateOff: "Not enabled",
    capStateConflict: "Backend conflict",
    capStatePartial: "Needs repair",
    // System base packages, collapsed appendix (Plan 1: physically separated from user extensions).
    // Accessible name for the row's `···` overflow menu (many rows per screen: it must
    // say which row it belongs to).
    rowMoreActions: (pkg: string) => `${pkg}: more actions`,
    // When the capability catalog fails to load (e.g. WSL guest profiles), the plugin
    // list says so instead of silently showing experimental packages as third-party.
    capsTagUnavailable:
      "Failed to read the experimental capability catalog: Experimental markers are temporarily unavailable (other markers are unaffected)",
    // Layer descriptions: the same set as the retired Base Bundles tab (2026-09-17:
    // agent-team layers were once described as "web UI renderer").
    bundleDescBase: "The Cordis base and its shared service plugins",
    bundleDescWebApp: "The web UI and interaction console renderer",
    bundleDescShipped: "A layer shipped with the dsh installation: switch it on the dsh plugins page",
    tabPatch: "Patch YAML",
    tabMcp: "MCP Extensions",
    mcpTitle: "MCP Server Manager",
    mcpSubtitle: "Manage Model Context Protocol external tools and services (with activation scope)",
    mcpEmpty: "No MCP servers configured for this profile yet",
    mcpAddBtn: "Add MCP Server",
    mcpLoadFailed: "Failed to load MCP configuration",
    mcpRetry: "Retry",
    mcpEditBtn: "Edit",
    mcpDeleteBtn: "Delete",
    mcpDeleteConfirm: (name: string) => `Remove MCP server "${name}"?`,
    // Activation scope (2026-09-18; verified in dsh-app-boot: profile layer → global layer → overlays)
    mcpScopeProfile: "This profile only",
    mcpScopeProfileHint:
      "Written to profiles/<name>/cordis.patch.yml: only this profile loads it.",
    mcpScopeGlobal: "All profiles",
    mcpScopeGlobalHint:
      "Written to $DSH_HOME/cordis.patch.yml: every profile loads it on startup.",
    mcpScopeLabel: "Activation scope",
    mcpScopeConflict: (name: string, rank: number) =>
      `"${name}" exists in both "This profile only" and "All profiles"; this row is #${rank} in load order: both rows are inserted, and the later one fails to instantiate because the serverName is already reserved. Delete one of them.`,
    mcpScopeConflictTag: (rank: number) => `duplicate across layers · #${rank} in load order`,
    mcpDupSameScope: (name: string, rank: number) =>
      `This layer has two rows named "${name}"; this one is #${rank}: both are inserted, and the later one fails to instantiate because the serverName is already reserved. Disable or delete the extra one (the toggle, probe and delete all act on this row only).`,
    mcpDupSameScopeTag: (rank: number) => `duplicate in this layer · #${rank}`,
    // !!js expression rows (2026-09-18; the long wording moved into the hover tip 2026-09-19)
    mcpExprTag: "has expression",
    mcpExprShort: "contains an !!js expression: only enable/disable is allowed",
    mcpExprHint:
      "This row's value is an !!js expression (the secret comes from an environment variable); what you see here is its flattened text, not what you would type. A structured save would flatten it into a literal and break the secret reference, so only enable/disable is allowed here — to change its configuration, edit that layer's cordis.patch.yml in a text editor.",
    // Secret display (env / headers usually carry tokens; masked by default)
    mcpSecretReveal: "Click to reveal this value",
    mcpSecretHide: "Click to hide this value",
    // MCP capability probe (2026-09-15, ADR-0022 stdio branch)
    mcpProbeBtn: "Probe",
    mcpProbeResult: (srv: string, tools: number, resources: number, templates: number) =>
      `"${srv}" capabilities: Tools ${tools} · Resources ${resources} · Templates ${templates}`,
    mcpProbeFailed: (srv: string, reason: string) => `Probe failed for "${srv}": ${reason}`,
    // 2026-09-15 (R1b): probe results render as an inline collapsible card.
    mcpProbeToggle: "Expand or collapse the capability list",
    mcpProbeAt: (time: string) => `snapshot ${time}`,
    mcpProbeProtocol: "Protocol",
    mcpProbeTools: "Tools",
    mcpProbeResources: "Resources",
    mcpProbeTemplates: "Resource templates",
    // SSH remote workspace wizard (2026-09-15, ADR-0023)
    mcpDeleteNote: "Removed from the cordis.patch.yml of the layer it lives in (backed up first).",
    mcpDeleteRowId: (rowId: string) =>
      `This layer has more than one row with that name; the row being deleted is the one with id "${rowId}".`,
    // Assembly state (2026-09-18): configured ≠ loaded. `enabled` is the config-level
    // value; `fiberPhase` is what tells you whether a live instance exists.
    // 2026-09-19: the row shows only the `*Tag` conclusion; the four sentences below
    // became the hover content.
    mcpRuntimeActiveTag: "loaded",
    mcpRuntimeDisabledTag: "disabled at runtime",
    mcpRuntimeFailedTag: "load failed",
    mcpRuntimeNotLoadedTag: "not loaded",
    mcpRuntimeActive: "Loaded in this session: the tools are available to the model",
    mcpRuntimeDisabled: "Runtime reports this row as disabled (configuration is not in effect)",
    mcpRuntimeFailed: "Runtime reports a load failure: check that row's error in the startup log",
    mcpRuntimeNotLoaded: (phase: string | null) =>
      phase
        ? `Runtime reports ${phase}: not ready yet`
        : "Runtime reports no live instance for this row (enabled but never loaded; usual causes: command unreachable, missing dependency, or a session restart is needed)",
    mcpModalTitle: "Configure MCP Server",
    mcpModalDesc:
      "Configure the server name, activation scope, command, arguments and environment variables; saved into the matching layer's cordis.patch.yml.",
    mcpPresetTitle: "Quick Apply Preset",
    mcpServerName: "Server Name",
    mcpNameHint:
      "It becomes part of the tool name mcp__<name>__<tool>, so only letters, digits, underscore and hyphen are allowed, up to 32 characters (a localized name makes the whole plugin row fail to load).",
    mcpNameInvalid: (name: string) =>
      `"${name}" violates the upstream constraint: letters, digits, underscore and hyphen only, up to 32 characters.`,
    mcpNameTaken: (name: string) =>
      `The selected scope already has a "${name}" row: saving as new would overwrite it rather than add a second one. Change the name to add another, or use Edit to change that one.`,
    // Transport (2026-09-18): the two branches take completely different fields
    mcpTransportLabel: "Transport",
    mcpTransportStdio: "Local command (stdio)",
    mcpTransportHttp: "Remote endpoint (streamable-http)",
    mcpTransportStdioHint:
      "dsh spawns a local child process and talks over stdin/stdout: fill in command, arguments and environment variables.",
    mcpTransportHttpHint:
      "Connects to an endpoint that already runs: fill in the URL and request headers, no command needed.",
    mcpUrl: "MCP endpoint URL",
    mcpUrlHint:
      "Like http://127.0.0.1:8000/mcp or https://example.com/mcp; it must start with http(s)://.",
    mcpUrlRequired: "streamable-http requires an MCP endpoint URL",
    mcpUrlMissing: "This row has no endpoint URL (it cannot load)",
    mcpHeaders: "Request Headers",
    mcpHeadersLabel: "Headers",
    mcpAddHeader: "+ Add header",
    mcpHeaderRemove: "Remove this request header",
    mcpNoHeaders: "No extra request headers",
    mcpCommand: "Command",
    mcpArgs: "Arguments (Args)",
    mcpCwd: "Working directory (cwd)",
    mcpCwdHint:
      "Startup directory for the stdio child process; may be left empty. Required by the \"absolute node path + absolute entry script\" form (which avoids PATH entirely) so the script can resolve its own dependencies.",
    mcpCommandHint:
      "The command is resolved by the dsh child process, whose PATH is much narrower than your login shell: the engine directory bundled with the dock ships pnpm only (there is no npx). So run a package on the fly with \"pnpm dlx\" rather than npx, and for something installed locally use an absolute path (e.g. /usr/bin/node) plus its directory as the working directory below.",
    mcpArgsHint:
      "Space-separated; quote a path that contains spaces so it stays one argument, e.g. \"/Program Files/node/server.js\".",
    mcpEnv: "Environment Variables (KEY=VALUE per line)",
    mcpSaveBtn: "Save Server",
    mcpSaveSuccess: (name: string) => `MCP server "${name}" saved to cordis.patch.yml`,
    mcpDeleteSuccess: "MCP server removed successfully",
    emptySelectTitle: "No Profile Selected",
    emptySelectSubtitle: "Select a profile from the left or create a new workbench to manage plugins, base bundles, and YAML patch.",
    quickDistribute: "Distribute to...",
    distributeTitle: (pkg: string) => `Distribute "${pkg}" to other profiles`,
    distributeNote: "Select target profile to install same version and optionally migrate configuration",
    distributeDone: (pkg: string, target: string) => `Successfully installed "${pkg}" into "${target}"`,
    distributeConfigFailed: (pkg: string, target: string, reason: string) =>
      `"${pkg}" was installed into "${target}", but configuration migration failed: ${reason}`,
    distributeConfigSkipped: (pkg: string, target: string) =>
      `"${pkg}" was installed into "${target}"; target already had matching config lines, left untouched`,
    copyPatchSuccess: "Patch YAML copied to clipboard",
    rawYamlHint: "Authoritative cordis.patch.yml generated by DSH (read-only diagnostics)",
    filterAll: "All",
    filterMaterialized: "Created",
    filterTemplates: "Templates",
    // ==== 2026-09-18 Profiles UI convergence: migrated inline copy ====
    actionDetail: "View details",
    searchEmpty: "No matching Profile found",
    mcpLoading: "Reading MCP server config...",
    mcpEmptyHint: "One-click presets available for GitHub, Postgres, Brave Search and other official MCP servers.",
    mcpEnabledTag: "Enabled",
    mcpNameRequired: "Please enter a server name",
    mcpCopyPrefix: "Copy tool prefix",
    mcpPrefixCopied: (prefix: string) => `Tool prefix copied: ${prefix}`,
    mcpApplyPreset: "Apply preset",
    mcpAddEnv: "+ Add variable",
    mcpNoEnv: "No environment variables needed",
    mcpNamePlaceholder: "e.g. github, filesystem, postgres",
    mcpPresetDescs: {
      github: "Search code, manage issues, PRs and commit history",
      filesystem: "Sandboxed local file read/write within chosen directories",
      postgres: "Connect to PostgreSQL and run safe read/write SQL",
      "brave-search": "Real-time web search via the Brave Search API",
    },
    overviewAllProfiles: "All Profiles",
    overviewCount: (n: number) => `${n} plugins in total`,
    overviewPerPage: (n: number) => `${n} / page`,
    overviewScanLoading: "Scanning plugin matrix across profiles...",
    overviewEmptySearch: "No matching plugins",
    overviewEmptySearchHint: "Try adjusting the search keyword or resetting the Profile filter.",
    overviewLoadFailed: "Failed to load plugins across profiles",
    overviewNoDesc: "No plugin description yet",
    overviewInstalledTo: (n: number) => `Installed in (${n}):`,
    overviewInstalledOn: (name: string) => `Installed in ${name}`,
    overviewRefCount: (n: number) => `${n} references`,
    overviewPrevPage: "Prev",
    overviewNextPage: "Next",
    overviewPageInfo: (i: number, total: number) => `Page ${i} / ${total}`,
    expandAllSources: (n: number) => `Show all (${n})`,
    collapseSources: "Collapse",
    distributePickPlaceholder: "Choose target Profile...",
    distributeAllInstalled: "All materialized Profiles already have this plugin",
    distributeTargetVersion: "Target version",
    distributeWithConfigDesc: "Copies the config entry verbatim from the first source Profile's cordis.patch.yml",
    distributeEnqueueBtn: "Add to distribute queue",
    importConfigCopyFailed: (reason: string) => `Plugin installed, but config copy failed: ${reason}`,
    // ==== 2026-09-18 Profiles 区 UI 收口：ProfileDetailPane 内联硬编码中文迁入 ====
    manifestNameLabel: "Package: ",
    detailLoading: "Loading profile...",
    // Unmaterialized profile (no directory yet: created on first launch or first plugin add).
    // 2026-09-21 real-machine bug: all four reads necessarily fail for such a profile, and
    // the capability catalog one surfaced the raw OS error to the user. Now the page issues
    // no requests at all and shows this one sentence plus the only way out (launch it).
    notMaterializedTag: "Not created",
    notMaterializedTitle: (name: string) => `Profile "${name}" does not exist on disk yet`,
    notMaterializedBody:
      "There is no working directory yet: built-in template names (web / headless) are created on first launch or on the first plugin add. The plugin list, experimental capabilities and configuration appear here afterwards.",
    searchNoPlugin: "No installed plugins match the search.",
    desktopRuntimeName: "DeepSeek official desktop client runtime",
    desktopRuntimeTag: "Official desktop",
    desktopRuntimeNote:
      "Note: these core components are deployed and maintained uniformly by the client runtime; they belong to the built-in base layer and cannot be uninstalled. They are listed below with the other dsh built-in layers, marked Built-in. Your own third-party and experimental-capability plugins are listed alongside them, each with its own marker.",
    copyYaml: "Copy YAML",
  },
  console: {
    toastClose: "Dismiss notification",
    copied: "Copied",
    logsLoading: "Reading log stream…",
    diagNodeTitle: "Node.js runtime",
    diagPnpmTitle: "pnpm package manager",
    diagDshTitle: "DSH core engine",
    diagStorageTitle: "Storage overview",
    diagNotDetected: "Not detected",
    diagNoPath: "No path",
    diagPnpmGlobalReady: "Ready (global)",
    diagPnpmMissing: "Missing",
    diagDshOfficialReady: "Official source (ready)",
    diagDshOfficialMissing: "Official source (not detected)",
    storageDistributionTitle: "DSH_HOME storage distribution",
    storageProfilesLabel: "Profile workbenches",
    storageSessionsLabel: "Session data",
    storageCacheOtherLabel: "Cache & other",
    navLabel: "System console navigation",
    detailLabel: "System console detail area",
    pullLatestLogs: "Refresh logs",
    rereadCredentials: "Refresh credentials",
    title: "System Console & Diagnostics",
    subtitle: "Preferences, LLM credentials, DSH engine settings, auto-recovery guardian, telemetry, and live logs",
    tabPreferences: "Preferences & Guardian",
    tabCredentials: "Model Credentials",
    tabDshSettings: "DSH Engine Settings",
    tabDiagnostics: "Health Telemetry",
    tabLogs: "Live Logs",
    // 凭据安全管理（4.5）
    credentialsTitle: "LLM & Service Credentials",
    credentialsSubtitle: "Securely manage $DSH_HOME/.credentials.yaml (atomic writes + masked storage)",
    configuredProviders: "Configured Providers",
    availableProviders: "Available Model Providers",
    configuredTag: "Configured",
    notConfiguredTag: "Not Configured",
    editKey: "Set Key",
    deleteKey: "Clear",
    keyModalTitle: (p: string) => `Configure ${p} API Key`,
    keyModalDesc: "Enter new API Key. Atomic write to .credentials.yaml (keeps 0600 mode on Unix).",
    // Accessible name for the input (placeholder "sk-..." is not a label, batch B2)
    keyInputLabel: "API Key",
    keySaved: "API Key updated successfully",
    keyRemoved: "API Key cleared",
    keyClearConfirm: (label: string) => `Clear the API Key for "${label}"?`,
    rawYamlToggle: "Raw YAML Mode",
    keyMasked: "Masked",
    toggleMask: "Show Plaintext",
    hidePlain: "Mask Secrets",
    saveCredentials: "Save Credentials",
    credentialsOverwriteConfirm: "Overwrite .credentials.yaml?",
    credentialsOverwritePoints: [
      "Rewrites the whole file with the editor's current content (not a merge)",
      "The previous file is backed up first as .credentials.yaml.bak-<timestamp>",
      "The write keeps atomic replacement (and 0600 mode on Unix)",
    ],
    credentialsSaved: "Credentials saved securely",
    credentialsSaveFailed: "Failed to save credentials",
    credentialsEmpty: "No credentials configured yet (file empty or not created)",
    insertTemplate: "Insert Template",
    permHint:
      "Security: on Unix this file uses 0600 mode (read/write by the current user only); on Windows it relies on the user profile's default ACL (not explicitly tightened)",
    // DSH 引擎全局设置
    dshSettingsTitle: "DSH Engine Global Configuration",
    dshSettingsSubtitle: "Manage $DSH_HOME/settings.yaml core runtime policies and global defaults",
    dshSettingsSaved: "DSH settings saved successfully",
    dshSettingsSave: "Save Configuration",
    dshSettingsOverwriteConfirm: "Overwrite settings.yaml?",
    dshSettingsOverwritePoints: [
      "Rewrites $DSH_HOME/settings.yaml with the editor's current content (not a merge)",
      "The previous file is backed up first as settings.yaml.bak-<timestamp>",
      "Broken content directly affects dsh startup — refresh and compare before saving",
    ],
    preferencesSection: "Client Preferences",
    localeLabel: "Interface Language",
    localeDesc: "Choose display language for the desktop shell UI; updates immediately",
    // 2026-09-11（task-18）：原值 `"System Default (跟随系统)"` 夹带中文，en 用户可见
    // ⇒ 漏译缺陷。语义对齐 zh 侧 `console.localeSystem`「跟随系统语言」；此处采用
    // 语言选择器通行标签 `System Default`（与 zh 侧括注一致），保持纯英文。
    // `localeZh` 为语言自名（endonym）例外：en 语境下以「简体中文」书写是正确的，
    // 门禁 `enUsNoLeak.test.ts` 的例外表内已显式登记理由。
    localeSystem: "System Default",
    localeZh: "简体中文 (Chinese Simplified)",
    localeEn: "English (US)",
    // Locale card subtitles (2026-09-11, task-20; corrected task-22): moved out
    // of the component (were hardcoded Chinese there). The zh card's "Default"
    // claim was REMOVED — the product default is "follow system"
    // (`preference: "system"` in i18nStore.ts; `locale: Option<String>` defaults
    // to None in settings.rs), so labelling Simplified Chinese as the default
    // lied about the default language for any non-zh system locale. Only the
    // system card keeps a subtitle, and that one is true and useful.
    localeSystemHint: "Auto Detect",
    // Launch environment on next start (v1.2.0 report 1.3). Values map 1:1 to the
    // Rust `defaultMode`: null = ask every time; "local" / "wsl" = start directly.
    bootModeSection: "Launch Environment on Next Start",
    bootModeDesc: "Pick the environment the app enters by default at each launch. This affects startup only; the running session is untouched.",
    bootModeAsk: "Ask Every Time",
    bootModeAskHint: "Show the environment picker at startup",
    bootModeLocal: "Run Locally",
    bootModeLocalHint: "Start directly in the local environment",
    bootModeWsl: "Inside WSL2",
    bootModeWslHint: "Start directly inside the WSL2 distro",
    bootModeFallbackHint: "If the local environment fails to initialise (e.g. no symlink privilege on Windows), set the default to \"Inside WSL2\" to work around it.",
    guardianSection: "High Availability & Crash Guardian",
    autoRestartLabel: "Auto-Recovery Guardian",
    autoRestartDesc: "Automatically restarts and recovers DSH session upon unexpected process exit. If 3 crashes occur within 60 seconds, circuit-breaker triggers automatically to prevent restart loops.",
    autoRestartEnabled: "Guardian active (with 3 crashes / 60s circuit breaker)",
    autoRestartDisabled: "Guardian disabled (manual retry by default)",
    // Circuit-breaker diagram (2026-09-11, task-20): was hardcoded Chinese in the
    // component; units differ per locale ("60s" vs 「60 秒」), so labels and values
    // both live here.
    breakerTitle: "Smart Circuit Breaker Protocol",
    breakerWindowLabel: "Window:",
    breakerWindowValue: "60s sliding window",
    breakerThresholdLabel: "Trip threshold:",
    breakerThresholdValue: "3 crashes in a row",
    breakerActionLabel: "On trip:",
    breakerActionValue: "Stop and show diagnostics",
    // Plugin install registry (ADR-0006 §6, 2026-09-16): mirrors and the official
    // registry each have gaps, so auto-switching is the default.
    pluginRegistrySection: "Plugin install registry",
    pluginRegistryDesc:
      "Where plugin packages are fetched from. Each source has gaps: mirrors may lag behind new releases, while the official registry can be slow or unreachable in some networks.",
    pluginRegistryAuto: "Automatic (recommended)",
    pluginRegistryAutoHint:
      "Tries the official registry first, switches to the other source on failure, and remembers whichever worked.",
    pluginRegistryOfficial: "Official registry only",
    pluginRegistryOfficialHint: "registry.npmjs.org: the most complete, but can be slow or unreachable.",
    pluginRegistryConfigured: "Configured source only",
    pluginRegistryConfiguredHint:
      "Uses whatever your npm config points at (e.g. npmmirror): fast, but may miss freshly published packages.",
    pluginRegistryOfficialShort: "official registry",
    pluginRegistryConfiguredShort: "configured source",
    pluginRegistryLastGood: (name: string) => `Last worked: ${name}`,
    switcherSection: "Workbench Quick Switcher & Floating Pill",
    floatingSwitcherLabel: "Workbench Floating Pill",
    floatingSwitcherDesc: "Display quick switcher capsule at top-center in DSH workbench (supports mouse drag; shortcuts remain active when disabled)",
    floatingSwitcherEnabled: "Floating pill enabled",
    floatingSwitcherDisabled: "Floating pill hidden (shortcut only)",
    shortcutLabel: "Control Center Shortcut",
    shortcutDesc: "Shortcut to toggle between DSH Workbench and Control Center (works in-app; the window must be focused)",
    shortcutDefault: "Default (⌘ + , / Ctrl + ,)",
    shortcutShiftP: "Command Palette Style (⌘ + ⇧ + P / Ctrl + ⇧ + P)",
    saveSuccess: "Settings saved",
    saveFailed: "Failed to save settings",
    // Saving is disabled when the read fails: settings.json is written as a
    // whole, so writing back without a real baseline would wipe other keys
    // (2026-09-08, see lib/shellSettings.ts)
    settingsLoadFailed:
      "Failed to read preferences — saving is disabled to avoid overwriting other settings",
    settingsLoading: "Loading preferences…",
    retryLoad: "Retry",
    diagnosticsLoadFailed: "Failed to collect diagnostics",
    diagnosticsTitle: "Runtime Environment Health",
    diagnosticsSubtitle: "Real-time runtime telemetry · Resolved paths & storage distribution",
    refreshDiagnostics: "Refresh Telemetry",
    nodeCard: "Node.js Runtime",
    pnpmCard: "pnpm Package Manager",
    dshCard: "DSH Core Engine",
    storageCard: "Storage Distribution",
    statusReady: "Ready",
    statusMissing: "Missing",
    sourceLabel: "Origin",
    pathLabel: "Resolved Path",
    versionLabel: "Version",
    copyReport: "Copy Diagnostics Report",
    reportCopied: "Diagnostics report copied to clipboard",
    profilesUsage: (n: number, size: string) => `${n} Profiles · ${size}`,
    sessionsUsage: (n: number, size: string) => `${n} Sessions · ${size}`,
    totalUsage: (size: string) => `Total Storage: ${size}`,
    logsTitle: "Live Log Viewer",
    logsSubtitle: "Multi-source log aggregation and troubleshooting",
    sourceShell: "DSH Dock Shell Log",
    sourceDsh: "DSH Runtime Log",
    searchPlaceholder: "Filter log messages...",
    autoScroll: "Auto-Scroll",
    copyLogs: "Copy Logs",
    logsCopied: "Logs copied to clipboard",
    clearLogs: "Clear Screen",
    totalLines: (n: number) => `Total ${n} lines`,
    truncatedHint: "Showing latest 500 lines",
    emptyLogs: "No log entries available",
    // 2026-09-18 UI convergence: copy moved out of CredentialsPane /
    // DshSettingsPane (was hardcoded Chinese there). permMaskNote keeps the
    // leading "." because the existing permHint value stays untouched.
    cardView: "Card View",
    permTitle: "File Permissions & UI Masking",
    permMaskNote: ". The UI never holds full plaintext API keys — only masked values are shown.",
    keyNotConfigured: "No API key configured",
    saveApiKey: "Save API Key",
    editKeyExisting: "Edit Key",
    copyYaml: "Copy YAML",
    reload: "Reload",
    yamlFormat: "YAML format",
  },
  // Community Plugin Marketplace
  // 2026-09-11: no hardcoded plugin counts in fixed copy (loading state cannot
  // know the real count) — see zh-CN.ts for the ruling; count-bearing copy stays
  // a function fed by the real registry data.
  market: {
    clearSearch: "Clear search",
    title: "Plugin Marketplace",
    subtitle: "Community plugins and extensions from the awesome-dsh-plugin registry",
    searchPlaceholder: "Search plugins by name, description, npm package, or author...",
    allCategories: "All Categories",
    // Category option in the filter dropdown: same "label + count" shape as the market pills.
    categoryOption: (label: string, n: number) => `${label} (${n})`,
    categoryCount: (n: number) => `${n} Categories`,
    totalPlugins: (n: number) => `${n} Plugins`,
    sortStars: "Most Stars",
    sortDownloads: "Most Downloads",
    sortNewest: "Recently Added",
    sortName: "Alphabetical",
    filterInstalled: "Installed Only",
    filterAll: "All Plugins",
    installBtn: "Install",
    reinstallBtn: "Reinstall",
    installToBtn: (prof: string) => `Install to ${prof}`,
    distributeBtn: "Distribute",
    installedIn: (count: number) => `Installed in ${count} profiles`,
    installedBadge: "Installed",
    installedInProfile: (prof: string) => `Installed in ${prof}`,
    installedWillOverwrite: "Already installed in this profile (will overwrite/reinstall)",
    installedWillReinstall: "Already installed in this profile (will be overwritten and reinstalled)",
    notInstalled: "Not Installed",
    noDescription: "No description",
    officialCoreTitle: "DSH official core plugin",
    // Experimental capability switches (2026-09-16, ADR-0020 §7: catalog rows -> capability switches)
    // Copy discipline (after the 2026-09-17 v3 layout rework): the left-hand list row carries only
    // what must be known at a glance (name / state / providing plugin / switch). Value, plugin
    // list, prerequisites, pinned versions and row ids all live in the right-hand always-on
    // detail pane — nothing there needs expanding any more, so it is no longer "collapsed copy".
    capTitle: "Experimental Capabilities",
    // 2026-09-17 ruling: make the official provenance explicit; package names are the official
    // names and each description is the package's own (`description` from its package.json, read
    // locally once installed — never paraphrased or translated).
    capOfficialBadge: "DSH Official",
    capOfficialNote:
      "These are dsh experimental capabilities published by DeepSeek (@deepseek-ai/* packages): dsh-dock only curates them and flips the switch — they are not community plugins.",
    // Two-pane layout (list / detail pane): list heading, detail-pane accessible name, and the
    // back control used by the narrow-window drill-down.
    capListLabel: "Capabilities",
    capPaneLabel: (name: string) => `${name} details`,
    capPluginsLabel: "Plugins",
    // "Switch backend" overflow menu (v4, at the end of each rail row): accessible name for
    // the trigger and the menu heading.
    capMenuBackendLabel: "Choose a backend",
    capBackendExclusiveNote:
      "Backends of one capability are mutually exclusive — switching hands over",
    // Marks the menu item that is currently active (clicking it is a no-op; the item is
    // disabled anyway).
    capVariantActive: "Active",
    capSwitchToThis: "Switch to this",
    capBasePackages: (pkgs: string) => `shared base ${pkgs}`,
    capAlsoInstalls: (pkgs: string) => `also installs shared ${pkgs}`,
    capDesc: "dsh capabilities that upstream marks as experimental. Turn one on to install and mount it; you can turn it off at any time.",
    // Single tooltip text for a capability (summary = what it is; unlocks = what you get).
    // 2026-09-21 third revision: both used to be laid out in the expanded body (one
    // paragraph plus a titled section), pushing "pick a backend" below the fold.
    capTipText: (summary: string, unlocks: string) => `${summary} When enabled: ${unlocks}`,
    capLoadFailed: "Failed to load experimental capabilities",
    capReload: "Retry",
    capSummary: (on: number, total: number) => `${on} of ${total} enabled`,
    capSummaryNeedsWork: (n: number) => `${n} need attention`,
    // "Disabled" is a state the user chose (turned off without uninstalling) — not a problem.
    capSummaryDisabled: (n: number) => `${n} disabled`,
    capSummaryOff: (n: number) => `${n} not enabled`,
    capRestartHint: "Configuration changed. Restart this Profile to apply it.",
    capRestartNow: "Restart Now",
    capStateOn: "Enabled",
    capStateOff: "Not Enabled",
    capStateDisabled: "Disabled",
    capStatePartial: "Needs Repair",
    capStateConflict: "Backend Conflict",
    capSwitchLabel: (name: string) => `Toggle ${name}`,
    capOtherActive: (label: string) => `"${label}" is currently active; turning this plugin on replaces it first`,
    capOtherReady: (label: string) => `"${label}" is already in place (disabled); turning this plugin on removes it first`,
    capOffIsRemove: "Provided by a profile layer: turning it off removes it",
    capStateSubsumed: "Included",
    capSubsumedBy: (label: string) =>
      `This backend's packages are the base layer of "${label}" and are already in place; "${label}" controls the toggle and removal`,
    capRunning: (index: number, total: number) => `Working (step ${index} of ${total})`,
    capOpInstall: (pkg: string) => `Install ${pkg}`,
    capOpRemove: (pkg: string) => `Remove ${pkg}`,
    capOpEnsureRow: (pkg: string) => `Add config row for ${pkg}`,
    capOpDeleteRow: (pkg: string) => `Clean up config row for ${pkg}`,
    capOpDisableRow: "Disable config row",
    capOpEnableRow: "Enable config row",
    capFailed: "Not all steps completed",
    capFailNetwork:
      "Neither package source could be reached (network hiccup, proxy, or timeout). Check the network and retry.",
    capFailNotFound:
      "Neither source has this package or version — check the name/version, or retry later (mirror sync lags).",
    capFailBuildApproval:
      "pnpm blocked a build script: approve it in the profile's pnpm-workspace.yaml, then retry.",
    capFailUnknown: "Install failed — expand the raw output for the exact reason.",
    capFailRawToggle: "Raw output",
    capResume: "Continue remaining steps",
    capQueueHint: "Package downloads and retries live in Downloads",
    capRepair: "Repair",
    capReadyToEnable: "Ready to turn on",
    capPinned: (spec: string) => `Pinned to ${spec}`,
    capRemoveBtn: "Remove and uninstall",
    capRemoveExplainedSoft: "Turning it off only disables the config row and keeps the downloaded packages. Only a full removal uninstalls them.",
    capRemoveExplainedLayer: "This capability comes from a profile layer, so turning it off removes it, including the entry in the layer list.",
    capCancel: "Cancel",
    capConfirmTitle: (name: string) => `Turn on "${name}"?`,
    capConfirmNote: "This installs the packages below and writes them into this Profile's config. It may take a few minutes.",
    capConfirmStart: "Turn On",
    capNoPrereq: "No extra prerequisites",
    capReplaceFrom: (label: string) => `Removes the currently active "${label}" backend first (its packages and config rows are uninstalled, and it is not restored automatically)`,
    capRemoveTitle: (name: string) => `Remove "${name}"?`,
    capRemoveNote: "This is a destructive action and cannot be undone.",
    capRemoveNoteLayer: "This capability comes from a profile layer, so turning it off means removing that layer; its packages get uninstalled.",
    capRemoveConfirm: "Remove",
    capRemovePointPackages: (n: number) => `Uninstalls ${n} packages (turning it back on downloads them again)`,
    capReplaceDisplaced: (pkgs: string) =>
      `The installed backend packages are removed first: ${pkgs} (not reinstalled automatically)`,
    capRemovePointRows: "Deletes the config rows written by dsh-dock (leaving them behind would strand rows pointing at missing packages)",
    capRemovePointLayer: "Removes this layer from the profile layer list",
    capDone: "Applied",
    // Punctuation and whole sentences live in the dictionary too: the component must not carry
    // full-width CJK punctuation (2026-09-17 independent review: switching to English showed
    // strings such as `Could not read experimental capabilities：…`).
    capValueSep: ": ",
    capWhyJoin: "; ",
    capDoneFor: (name: string) => `${name}: applied`,
    capPartialFor: (name: string, done: number, total: number) =>
      `${name}: ${done}/${total} done, the remaining steps can be resumed`,
    capFailedOn: (name: string) => `failed on ${name}`,
    capRowHasFailure: "This capability has an unresolved failure — the detail pane shows why",
    capPartial: (done: number, total: number) => `Finished ${done} of ${total}; you can continue the remaining steps`,
    installModalTitle: (pkg: string) => `Install Plugin "${pkg}"`,
    installModalDesc: "Choose target profile. It will be added to package.json and built via pnpm automatically.",
    selectProfile: "Select Target Profile",
    installSpecLabel: "Install Source",
    sourceNpm: "NPM Package",
    sourceGithub: "GitHub Repo",
    sourceAutoDetected: "Auto-detected specifier",
    installingBusy: "Installing via pnpm, this may take up to a minute…",
    installSuccess: (pkg: string, prof: string) => `Plugin "${pkg}" successfully installed to profile "${prof}"! Restart the profile to take effect.`,
    installFailed: (msg: string) => `Installation failed: ${msg}`,
    viewReadme: "GitHub",
    viewNpm: "NPM",
    openOfficialDoc: "Home",
    loadingRegistry: "Connecting to the community registry & loading plugins…",
    loadFailed: "Failed to load marketplace registry",
    retry: "Retry",
    noResults: "No matching plugins found",
    noResultsHint: "Try changing your search query or clearing the category filter",
    clearFilters: "Clear all filters",
    paginationPrev: "Previous",
    paginationNext: "Next",
    pageInfo: (current: number, total: number, count: number) => `Page ${current} of ${total} (${count} items)`,
    pageSize: "Per page",
    cacheHit: "Cache Hit",
    refreshRegistry: "Refresh",
    loadingBtn: "Loading…",
    openLinkFailed: (msg: string) => `Failed to open link: ${msg}`,
    author: "Author",
    addedDate: "Added",
    subtabMarket: "Marketplace",
    subtabInstalled: "Installed",
    expandCategories: (n: number) => `Show All (${n})`,
    collapseCategories: "Show Less",
    manualInstallBtn: "Manual Install",
    manualInstallTitle: "Manual Plugin Installation",
    manualInstallDesc: "Enter an NPM package name, GitHub repository, or Tarball spec, and select the target Profile.",
    manualInstallSpecPlaceholder: "e.g. @deepseek-ai/dsh-pet or github:user/repo",
    manualInstallSubmit: "Install to Selected Profile",
    installQueued: (pkg: string, prof: string) => `Added to download queue: "${pkg}" -> ${prof}`,
    // 2026-09-15 (R2): uninstall shares the same queue — exclusive-family handover
    // must be visible and retryable in the panel; copy is kept separate from install.
    queueRemoveQueued: (pkg: string, prof: string) => `Added to download queue: uninstall "${pkg}" <- ${prof}`,
    queueRemoveDone: (pkg: string, prof: string) => `Uninstalled "${pkg}" from profile "${prof}"`,
    queueTitle: "Downloads",
    queueEmpty: "No download tasks",
    queueClearDone: "Clear finished",
    queueStatusQueued: "Queued",
    queueStatusInstalling: "Installing",
    queueStatusDone: "Installed",
    queueStatusRemoved: "Uninstalled",
    queueStatusFailed: "Failed",
    queueDismiss: "Dismiss",
    queueRetry: "Retry",
    queueFailedNotice: (pkg: string, detail: string) => `Failed to install "${pkg}": ${detail}`,
    queueRemoveFailedNotice: (pkg: string, detail: string) => `Failed to uninstall "${pkg}": ${detail}`,
  },
  // Shared copy for the destructive-action confirm dialog (2026-09-08, U9)
  confirm: {
    cancel: "Cancel",
  },
  // Explanatory hover tips (ui/info-tip.tsx, 2026-09-19): the icon carries no visible text.
  tip: {
    aria: "Show more information",
    ariaFor: (subject: string) => `More information about ${subject}`,
  },
}
