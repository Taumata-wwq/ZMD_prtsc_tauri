// 中文语言包（默认）
export const zh: Record<string, string> = {
  // 通用
  'common.loading': '加载中...',

  // 标题栏
  'titlebar.minimize': '最小化',
  'titlebar.close': '关闭',
  'titlebar.pin': '窗口置顶',
  'titlebar.themeToggleLight': '切换到浅色主题',
  'titlebar.themeToggleDark': '切换到深色主题',

  // 主导航
  'nav.capture': '截图',
  'nav.history': '历史',
  'nav.settings': '设置',
  'nav.data': '数据',

  // 截图视图
  'capture.waiting': '等待截图',
  'capture.start': '开始',
  'capture.stop': '停止',
  'capture.region': '区域',
  'capture.scrollMode': '滚动次数',
  'capture.gridSize': '网格大小（行 × 列）',
  'capture.logs': '日志',
  'capture.outputFormat': '输出格式',
  'capture.quality': '质量',
  'capture.selectDir': '选择...',
  'capture.exportOriginal': '导出原图',
  'capture.exportCropped': '导出裁剪',
  'capture.clearSelection': '清除选区',
  'capture.processing': '正在拼接图像...',
  'capture.exporting': '正在导出...',
  'capture.exportDone': '导出完成',
  'capture.hint.title': '使用说明',
  'capture.hint.items': '1. 调整游戏分辨率为 16:9\n2. 按 X 进入批量选择模式\n3. 上滚轮到紧贴地面后移动到左上角\n4. 截图时请勿移动鼠标',

  // 设置视图
  'settings.delayGroup': '延迟参数',
  'settings.dragGroup': '拖拽设置',
  'settings.outputGroup': '输出目录',
  'settings.about': '关于',
  'settings.language': '语言',
  'settings.language.zh': '中文',
  'settings.language.en': 'English',
  'settings.outputFolder': '输出目录',
  'settings.minimizeOnCapture': '截图时最小化窗口',
  'settings.minimizeOnCaptureHint': '开始截图后自动最小化软件窗口，将焦点让给游戏',
  'settings.outputFolderHint': '截图导出保存的目录路径',
  'settings.stabilizeDelayHint': '画面稳定等待时间（30-500ms）',
  'settings.screenshotDelayHint': '每张截图前的等待时间（30-500ms）',
  'settings.dragDurationHint': '相机拖拽持续时间（30-500ms）',
  'settings.languageHint': '切换界面显示语言',
  'settings.filenamePattern': '导出文件名格式',
  'settings.filenamePatternHint': '可用占位符：{region} 区域, {timestamp} 时间戳, {scrollMode} 滚动次数',

  // 关于
  'about.appName': '应用名',
  'about.version': '版本',
  'about.techStack': '技术栈',
  'about.license': '开源许可',

  // 延迟参数
  'delay.stabilize': '稳定延迟（秒）',
  'delay.screenshot': '截图间隔（秒）',
  'delay.dragDuration': '拖拽时长（秒）',
  'delay.dragMarginBottom': '横向拖拽距底边（像素）',
  'delay.dragMarginLeft': '纵向拖拽距左边（像素）',
  'settings.dragMarginBottomHint': '横向拖拽时鼠标光标距离游戏窗口底边的距离（像素）',
  'settings.dragMarginLeftHint': '纵向拖拽时鼠标光标距离游戏窗口左侧边的距离（像素）',

  // 历史视图
  'history.title': '历史记录',
  'history.refresh': '刷新',
  'history.clear': '清空',
  'history.confirmClear': '确认清空所有历史记录？此操作不可撤销。',
  'history.empty': '暂无历史记录',
  'history.loading': '加载中...',
  'history.loadFailed': '加载失败',
  'history.selectSession': '请选择左侧会话查看详情',
  'history.openLocation': '打开导出位置',
  'history.openOriginalLocation': '打开原图位置',
  'history.startTime': '开始时间',
  'history.endTime': '结束时间',
  'history.region': '区域',
  'history.scrollMode': '滚动模式',
  'history.grid': '网格大小',
  'history.totalShots': '总截图数',
  'history.status': '状态',
  'history.outputFormat': '输出格式',
  'history.jpgQuality': 'JPG 质量',
  'history.originalPath': '原图路径',
  'history.exportedPath': '导出路径',
  'history.unknownRegion': '未知区域',
  'history.defaultMode': '默认',
  'history.shots': '张',

  // 错误
  'error.settingsLoad': '设置加载错误',

  // 数据管理视图
  'data.title': '数据管理',
  'data.add': '新增区域',
  'data.delete': '删除',
  'data.save': '保存',
  'data.cancel': '取消',
  'data.confirmDelete': '确认删除此区域配置？',
  'data.empty': '暂无区域配置',
  'data.name': '区域名',
  'data.category': '类别',
  'data.customCategory': '自定义类别',
  'data.inputTarget': '请输入目标尺寸',
}
