// English language pack
export const en: Record<string, string> = {
  // Common
  'common.loading': 'Loading...',

  // Title bar
  'titlebar.minimize': 'Minimize',
  'titlebar.close': 'Close',
  'titlebar.pin': 'Pin on top',
  'titlebar.themeToggleLight': 'Switch to light theme',
  'titlebar.themeToggleDark': 'Switch to dark theme',

  // Main navigation
  'nav.capture': 'Capture',
  'nav.history': 'History',
  'nav.settings': 'Settings',
  'nav.data': 'Data',

  // Capture view
  'capture.waiting': 'Waiting for capture',
  'capture.start': 'Start',
  'capture.stop': 'Stop',
  'capture.region': 'Region',
  'capture.scrollMode': 'Scroll count',
  'capture.gridSize': 'Grid size (rows × cols)',
  'capture.logs': 'Logs',
  'capture.outputFormat': 'Format',
  'capture.quality': 'Quality',
  'capture.selectDir': 'Browse...',
  'capture.exportOriginal': 'Export original',
  'capture.exportCropped': 'Export cropped',
  'capture.clearSelection': 'Clear selection',
  'capture.processing': 'Stitching image...',
  'capture.exporting': 'Exporting...',
  'capture.exportDone': 'Export complete',
  'capture.hint.title': 'Instructions',
  'capture.hint.items': '1. Set game resolution to 16:9\n2. Press X to enter batch selection mode\n3. Scroll up to ground level and move to top-left corner\n4. Do not move mouse during capture',

  // Settings view
  'settings.delayGroup': 'Delay',
  'settings.dragGroup': 'Drag',
  'settings.outputGroup': 'Output Folder',
  'settings.about': 'About',
  'settings.language': 'Language',
  'settings.language.zh': '中文',
  'settings.language.en': 'English',
  'settings.outputFolder': 'Output folder',
  'settings.minimizeOnCapture': 'Minimize on capture',
  'settings.minimizeOnCaptureHint': 'Minimize app window after starting capture to give focus to the game',
  'settings.outputFolderHint': 'Directory path where screenshots are exported',
  'settings.stabilizeDelayHint': 'Stabilization wait time (30-500ms)',
  'settings.screenshotDelayHint': 'Wait time before each screenshot (30-500ms)',
  'settings.dragDurationHint': 'Camera drag duration (30-500ms)',
  'settings.languageHint': 'Switch the interface display language',
  'settings.filenamePattern': 'Export filename pattern',
  'settings.filenamePatternHint': 'Placeholders: {region}, {timestamp}, {scrollMode}',

  // About
  'about.appName': 'Application',
  'about.version': 'Version',
  'about.techStack': 'Tech stack',
  'about.license': 'License',

  // Delay parameters
  'delay.stabilize': 'Stabilize delay (s)',
  'delay.screenshot': 'Screenshot interval (s)',
  'delay.dragDuration': 'Drag duration (s)',
  'delay.dragMarginBottom': 'Horiz. drag to bottom (px)',
  'delay.dragMarginLeft': 'Vert. drag to left (px)',
  'settings.dragMarginBottomHint': 'Distance from mouse cursor to bottom edge during horizontal drag (px)',
  'settings.dragMarginLeftHint': 'Distance from mouse cursor to left edge during vertical drag (px)',

  // History view
  'history.title': 'History',
  'history.refresh': 'Refresh',
  'history.clear': 'Clear',
  'history.confirmClear': 'Clear all history? This cannot be undone.',
  'history.empty': 'No history yet',
  'history.loading': 'Loading...',
  'history.loadFailed': 'Load failed',
  'history.selectSession': 'Select a session to view details',
  'history.openLocation': 'Open export folder',
  'history.openOriginalLocation': 'Open original folder',
  'history.startTime': 'Start time',
  'history.endTime': 'End time',
  'history.region': 'Region',
  'history.scrollMode': 'Scroll mode',
  'history.grid': 'Grid size',
  'history.totalShots': 'Total shots',
  'history.status': 'Status',
  'history.outputFormat': 'Output format',
  'history.jpgQuality': 'JPG quality',
  'history.originalPath': 'Original path',
  'history.exportedPath': 'Exported path',
  'history.unknownRegion': 'Unknown region',
  'history.defaultMode': 'Default',
  'history.shots': 'shots',

  // Errors
  'error.settingsLoad': 'Settings load error',

  // Data management view
  'data.title': 'Data Management',
  'data.add': 'Add Region',
  'data.delete': 'Delete',
  'data.save': 'Save',
  'data.cancel': 'Cancel',
  'data.confirmDelete': 'Delete this region configuration?',
  'data.empty': 'No region configurations',
  'data.name': 'Name',
  'data.category': 'Category',
  'data.customCategory': 'Custom Category',
  'data.inputTarget': 'Please input target size',
}
