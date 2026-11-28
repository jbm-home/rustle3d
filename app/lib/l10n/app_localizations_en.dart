// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class LEn extends L {
  LEn([String locale = 'en']) : super(locale);

  @override
  String get languageTooltip => 'Language';

  @override
  String get themeLight => 'Light theme';

  @override
  String get themeDark => 'Dark theme';

  @override
  String get cancel => 'Cancel';

  @override
  String get create => 'Create';

  @override
  String get save => 'Save';

  @override
  String get apply => 'Apply';

  @override
  String get delete => 'Delete';

  @override
  String get next => 'Next';

  @override
  String get skip => 'Skip';

  @override
  String get start => 'Start';

  @override
  String get gotIt => 'Got it';

  @override
  String get cm => 'cm';

  @override
  String get keyCtrl => 'Ctrl';

  @override
  String get keyAlt => 'Alt';

  @override
  String get keyShift => 'Shift';

  @override
  String get keyDelete => 'Delete';

  @override
  String get startTitle => 'What will you make?';

  @override
  String get templateTShirt => 'T-shirt';

  @override
  String get templateTShirtBlurb => 'Front, back and short sleeves, in jersey.';

  @override
  String get templateShirt => 'Shirt';

  @override
  String get templateShirtBlurb =>
      'Long sleeves with cuffs and a stand collar.';

  @override
  String get templateTrousers => 'Trousers';

  @override
  String get templateTrousersBlurb =>
      'Straight legs from the waist to the ankle.';

  @override
  String get templateHoodie => 'Hoodie';

  @override
  String get templateHoodieBlurb =>
      'Jersey, long sleeves, a hood, gathered at the wrists and the hem.';

  @override
  String get templateJoggers => 'Joggers';

  @override
  String get templateJoggersBlurb =>
      'Jersey, pulled on: an elastic waist and elastic ankles.';

  @override
  String get templateDress => 'Dress';

  @override
  String get templateDressBlurb => 'Sleeveless, gently shaped, knee length.';

  @override
  String get templateStraightSkirt => 'Straight skirt';

  @override
  String get templateStraightSkirtBlurb =>
      'Close over the hips, from the waist to the knee.';

  @override
  String get templateALineSkirt => 'A-line skirt';

  @override
  String get templateALineSkirtBlurb =>
      'Flares gently from the hips to the hem.';

  @override
  String get templateToteBag => 'Tote bag';

  @override
  String get templateToteBagBlurb =>
      'Two pieces and four seams: the easiest start.';

  @override
  String get sectionNew => 'New';

  @override
  String get openFromDisk => 'Open…';

  @override
  String get open => 'Open';

  @override
  String allProjectsCount(int count) {
    return 'All projects ($count)';
  }

  @override
  String get projectsFolder => 'Projects folder';

  @override
  String get importProjectZip => 'Import .zip…';

  @override
  String get importProjectZipTip =>
      'Add a project someone sent you as a .zip archive';

  @override
  String get projectZipFiles => 'Project archives';

  @override
  String get projectOptions => 'Project options';

  @override
  String get renameProject => 'Rename…';

  @override
  String get exportProjectZip => 'Export as .zip…';

  @override
  String get deleteProject => 'Delete…';

  @override
  String get renameProjectTitle => 'Rename the project';

  @override
  String get rename => 'Rename';

  @override
  String deleteProjectTitle(String name) {
    return 'Delete “$name”?';
  }

  @override
  String get deleteProjectBody =>
      'The project folder and everything in it (pattern, body, exports) will be deleted from this computer. This cannot be undone.';

  @override
  String get deleteProjectConfirm =>
      'I understand that this project will be deleted for good';

  @override
  String projectDeleted(String name) {
    return 'Deleted “$name”.';
  }

  @override
  String projectExported(String name, String file) {
    return 'Exported “$name” to $file.';
  }

  @override
  String appVersion(String version) {
    return 'Version $version';
  }

  @override
  String get updatesLink => 'Updates and news on app.rustle3d.com';

  @override
  String get supportLink => 'Support Rustle3D';

  @override
  String get supportTitle => 'Rustle3D is free and open source';

  @override
  String get supportText =>
      'If it is useful to you, a donation helps cover its costs: hosting, test hardware and development time. Everything stays free either way.';

  @override
  String get supportDonate => 'Donate';

  @override
  String get supportLater => 'Not now';

  @override
  String get supportTurnOff => 'Don’t show again';

  @override
  String updateTitle(String version) {
    return 'Rustle3D $version is available';
  }

  @override
  String get updateText =>
      'Download it from app.rustle3d.com and install it over this version: your projects are kept.';

  @override
  String get updateDownload => 'Download';

  @override
  String get updateSkip => 'Skip this version';

  @override
  String get updatesTitle => 'Updates';

  @override
  String get updateAuto => 'Check for updates once a day';

  @override
  String get updateAutoHint =>
      'The app asks app.rustle3d.com for the latest version number, sending only its own version and system. It never downloads or installs anything by itself.';

  @override
  String get updateCheckNow => 'Check now';

  @override
  String get updateLatest => 'You have the latest version.';

  @override
  String get updateFailed =>
      'app.rustle3d.com could not be reached. Check your connection and try again.';

  @override
  String get updateCheck => 'Check for updates';

  @override
  String cannotOpen(String target) {
    return 'Could not open $target.';
  }

  @override
  String get blankProject => 'Blank project';

  @override
  String get blankProjectBlurb => 'Draw your own pieces.';

  @override
  String get untitled => 'Untitled';

  @override
  String get recent => 'Recent';

  @override
  String get recentEmpty =>
      'Your projects will appear here. They are saved as you work, in Documents › Rustle3D.';

  @override
  String get agoJustNow => 'just now';

  @override
  String agoMinutes(int count) {
    return '$count min ago';
  }

  @override
  String agoHours(int count) {
    return '$count h ago';
  }

  @override
  String get agoYesterday => 'yesterday';

  @override
  String agoDays(int count) {
    return '$count days ago';
  }

  @override
  String newGarmentTitle(String garment) {
    return 'New: $garment';
  }

  @override
  String get fieldName => 'Name';

  @override
  String get fieldBody => 'Body';

  @override
  String sizeMustBePositive(String size) {
    return '$size must be a positive number of cm.';
  }

  @override
  String get sizeChest => 'Chest';

  @override
  String get sizeChestHint => 'Around the fullest part of the chest';

  @override
  String get sizeWaist => 'Waist';

  @override
  String get sizeWaistHint => 'Around the narrowest part';

  @override
  String get sizeHips => 'Hips';

  @override
  String get sizeHipsHint => 'Around the fullest part';

  @override
  String get sizeLength => 'Length';

  @override
  String get sizeLengthHint =>
      'Neck to hem for tops, waist to hem for skirts, height of a bag';

  @override
  String get sizeSleeve => 'Sleeve';

  @override
  String get sizeSleeveHint => 'From the top of the shoulder';

  @override
  String get sizeWidth => 'Width';

  @override
  String get sizeWidthHint => 'Across the bag';

  @override
  String get sizeEase => 'Ease';

  @override
  String get sizeEaseHint => 'Room added to the body';

  @override
  String get pieceFront => 'Front';

  @override
  String get pieceBack => 'Back';

  @override
  String get pieceBackLeft => 'Back left';

  @override
  String get pieceBackRight => 'Back right';

  @override
  String get pieceSleeveLeft => 'Sleeve left';

  @override
  String get pieceSleeveRight => 'Sleeve right';

  @override
  String pieceNew(int number) {
    return 'Panel $number';
  }

  @override
  String pieceMirror(String name) {
    return '$name (mirror)';
  }

  @override
  String get body => 'Body';

  @override
  String get bodySubtitle =>
      'The 3D body is fitted to these measurements. Start from a size, then adjust.';

  @override
  String get bodyPreset => 'Preset';

  @override
  String presetWomen(String size) {
    return 'Women $size';
  }

  @override
  String presetMen(String size) {
    return 'Men $size';
  }

  @override
  String get bodyFeminine => 'Feminine body';

  @override
  String get bodyMasculine => 'Masculine body';

  @override
  String get measHeight => 'Height';

  @override
  String get measBust => 'Bust / chest';

  @override
  String measBetween(String measure, String min, String max, String unit) {
    return '$measure must be between $min and $max $unit.';
  }

  @override
  String get presetName => 'Preset name';

  @override
  String get saveAsPreset => 'Save as preset';

  @override
  String get deletePreset => 'Delete preset';

  @override
  String fittedBody(String height, String bust, String waist, String hips) {
    return 'Fitted body: height $height · bust $bust · waist $waist · hips $hips';
  }

  @override
  String get bodyMore => 'More measurements';

  @override
  String get bodyMoreHint =>
      'Estimated from the four above until you type your own.';

  @override
  String get bodyTorso => 'Torso';

  @override
  String get bodyArms => 'Arms';

  @override
  String get bodyLegs => 'Legs';

  @override
  String get measEstimated => 'estimated';

  @override
  String get measReset => 'Back to the estimate';

  @override
  String get measNeck => 'Neck';

  @override
  String get measNeckHint => 'Around the base of the neck';

  @override
  String get measShoulderWidth => 'Shoulder width';

  @override
  String get measShoulderWidthHint =>
      'Straight across the back, shoulder tip to shoulder tip';

  @override
  String get measShoulderLength => 'Shoulder length';

  @override
  String get measShoulderLengthHint =>
      'From the side of the neck to the shoulder tip';

  @override
  String get measBackWaistLength => 'Back waist length';

  @override
  String get measBackWaistLengthHint =>
      'From the nape down the spine to the waist';

  @override
  String get measFrontWaistLength => 'Front waist length';

  @override
  String get measFrontWaistLengthHint =>
      'From the side of the neck over the bust to the waist';

  @override
  String get measWaistHeight => 'Waist height';

  @override
  String get measWaistHeightHint => 'From the waist down to the floor';

  @override
  String get measHipHeight => 'Hip height';

  @override
  String get measHipHeightHint =>
      'From the fullest part of the hips down to the floor';

  @override
  String get measCrotchDepth => 'Crotch depth';

  @override
  String get measCrotchDepthHint =>
      'From the waist down to the crotch (seated: to the chair)';

  @override
  String get measArmLength => 'Arm length';

  @override
  String get measArmLengthHint =>
      'From the shoulder tip over the bent elbow to the wrist bone';

  @override
  String get measUpperArm => 'Upper arm';

  @override
  String get measUpperArmHint => 'Around the upper arm at its fullest';

  @override
  String get measWrist => 'Wrist';

  @override
  String get measWristHint => 'Around the wrist';

  @override
  String get measInsideLeg => 'Inside leg';

  @override
  String get measInsideLegHint =>
      'From the crotch down the inside of the leg to the floor';

  @override
  String get measThigh => 'Thigh';

  @override
  String get measThighHint =>
      'Around the thigh at its fullest, just below the crotch';

  @override
  String get measKnee => 'Knee';

  @override
  String get measKneeHint => 'Around the knee';

  @override
  String get measAnkle => 'Ankle';

  @override
  String get measAnkleHint => 'Just above the ankle bone';

  @override
  String measAvatarReaches(String value) {
    return 'The avatar measures $value: its body cannot go further.';
  }

  @override
  String get allProjects => 'All projects';

  @override
  String get export => 'Export';

  @override
  String get importPattern => 'Import';

  @override
  String get importTip =>
      'Import a pattern drawn elsewhere (DXF-AAMA, DXF, SVG, PDF, Seamly2D or Valentina), or a Seamly2D or Valentina measurement file';

  @override
  String importUnsupported(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count kinds of drafting step are not supported yet: the pieces using them were left out.',
      one: '1 kind of drafting step is not supported yet: the pieces using it were left out.',
    );
    return '$_temp0';
  }

  @override
  String get measurementsImported =>
      'The avatar takes the measurements from the file.';

  @override
  String garmentSource(String file) {
    return 'Drafted from $file';
  }

  @override
  String get garmentRedraft => 'Draft again for the avatar';

  @override
  String get garmentRedraftTip =>
      'Run the pattern again with the avatar\'s measurements; your changes to the pieces are kept';

  @override
  String get busyRedraft => 'Drafting the pattern again';

  @override
  String get importFiles => 'Patterns';

  @override
  String imported(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pieces imported',
      one: '1 piece imported',
    );
    return '$_temp0.';
  }

  @override
  String importIgnored(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count shapes were not part of a piece.',
      one: '1 shape was not part of a piece.',
    );
    return '$_temp0';
  }

  @override
  String get importUnitsTitle => 'What unit was this drawn in?';

  @override
  String get importUnitsBody =>
      'The file does not give its scale, so the pieces may be the wrong size. Choose the unit it was drawn in, or keep them as they are.';

  @override
  String get importKeep => 'Keep as is';

  @override
  String get unitMm => 'Millimetres';

  @override
  String get unitCm => 'Centimetres';

  @override
  String get unitIn => 'Inches';

  @override
  String get unitPt => 'Points (1/72 inch)';

  @override
  String get unitPx => 'Pixels (1/96 inch)';

  @override
  String get agents => 'Agents';

  @override
  String get agentsWaiting => 'Agents: waiting';

  @override
  String get agentConnected => 'Agent connected';

  @override
  String get agentsTip => 'AI agents: let one edit this project live';

  @override
  String get allowAgents => 'Allow AI agents';

  @override
  String get agentsTitle => 'Connect an AI agent';

  @override
  String get agentsIntro =>
      'An AI agent (an assistant that runs tools) can edit this project live: draft pieces, sew, drape, look at the result. It talks to Rustle3D over MCP; the steps below take a minute, once.';

  @override
  String get agentsStepAllow => 'Let agents in';

  @override
  String get agentsStepAdd => 'Add Rustle3D to your agent';

  @override
  String get agentsAddHint =>
      'Add this server to your agent\'s MCP servers, then restart the agent. It starts r3d, the command-line tool installed with the app.';

  @override
  String get agentsJsonHint =>
      'Most desktop assistants read this in their MCP settings file; merge it into the file\'s \"mcpServers\".';

  @override
  String get agentsExtensionHint =>
      'An extension file for Claude Desktop and other assistants that install MCP bundles (.mcpb): save it, then open it, or drop it into the assistant\'s Settings > Extensions. It runs r3d from where the app is installed, so it keeps working after an update; if you move the app, save it again.';

  @override
  String get agentsSaveExtension => 'Save rustle3d.mcpb…';

  @override
  String agentsExtensionSaved(String path) {
    return 'Saved: $path. Open it to install it in your assistant.';
  }

  @override
  String get agentsCopy => 'Copy';

  @override
  String get agentsNoR3d =>
      'r3d was not found next to the app (a build from source?): build it with cargo build -p r3d-cli and put its full path in place of r3d.';

  @override
  String get agentsStepAsk => 'Ask your agent';

  @override
  String get agentsAskHint =>
      'For example: \"Look at my project in Rustle3D and lengthen the sleeves by 3 cm.\" Each change shows here at once and can be undone.';

  @override
  String get agentsStateOff => 'Agents are not allowed';

  @override
  String get agentsStateWaiting => 'Waiting for an agent…';

  @override
  String get disconnectAgent => 'Disconnect the agent';

  @override
  String get exportSvg => 'Pattern, SVG at 1:1';

  @override
  String get exportPdfA4 => 'Pattern, PDF on A4 pages';

  @override
  String get exportPdfSingle => 'Pattern, PDF on one page';

  @override
  String get exportGlb => 'Garment in 3D, glTF (.glb)';

  @override
  String get exportGlbAvatar => 'Garment and body in 3D, glTF (.glb)';

  @override
  String get exportObj => 'Garment in 3D, OBJ';

  @override
  String get exportDxf => 'Pattern, DXF-AAMA (pattern software)';

  @override
  String exported(String file) {
    return 'Exported $file.';
  }

  @override
  String get learn => 'Learn';

  @override
  String get undoTip => 'Undo  ⌘Z';

  @override
  String get redoTip => 'Redo  ⇧⌘Z';

  @override
  String get tourPatternTitle => 'Your pattern';

  @override
  String get tourPatternText =>
      'The flat pieces, full size. Drag a corner or an edge to reshape them; **R** adds a piece.';

  @override
  String get tourGarmentTitle => 'The garment on the body';

  @override
  String get tourGarmentText =>
      '**Simulate** drapes it. Drag to turn around; **⇧-drag** a hem or a seam to change the pattern from here.';

  @override
  String get tourHelpTitle => 'Help is always here';

  @override
  String get tourHelpText =>
      '**Learn** has short how-tos and shows where things are; **Body** changes the measurements. **⌘Z** undoes anything.';

  @override
  String get toolSelect => 'Select';

  @override
  String get toolSelectHint =>
      'Click a piece; drag it to move it, its frame\'s corners to scale it, the round handle above it to turn it (⇧: 15° steps, or scale both ways at once); drag a corner or an edge to reshape it';

  @override
  String get toolRectangle => 'Rectangle';

  @override
  String get toolRectangleHint => 'Add a rectangular panel';

  @override
  String get toolPolygon => 'Polygon';

  @override
  String get toolPolygonHint => 'Click the corners of a new piece';

  @override
  String get toolCurve => 'Curve';

  @override
  String get toolCurveHint =>
      'Draw a piece: click for a corner, drag for a smooth point';

  @override
  String get toolEditCurve => 'Edit curves';

  @override
  String get toolEditCurveHint =>
      'Bend an edge, straighten it, or add a point on it';

  @override
  String get toolBend => 'Bend';

  @override
  String get toolBendHint => 'Click an edge, then drag its handles to bend it';

  @override
  String get toolStraighten => 'Straighten';

  @override
  String get toolStraightenHint => 'Click a curved edge to make it straight';

  @override
  String get toolAddPoint => 'Add point';

  @override
  String get toolAddPointHint => 'Click an edge where the new point goes';

  @override
  String get toolCut => 'Cut';

  @override
  String get toolCutHint =>
      'Draw a line across a piece: it becomes two pieces sewn together';

  @override
  String get toolDraw => 'Draw';

  @override
  String get toolDrawHint =>
      'Drag across a piece, on the pattern or on the 3D garment: the line goes on the pattern';

  @override
  String get toolNotch => 'Notch';

  @override
  String get toolNotchHint => 'Click an edge where the notch goes';

  @override
  String get toolSew => 'Sew';

  @override
  String get toolSewHint =>
      'Click two edges or lines on the pattern; drag along one to sew part of it';

  @override
  String get viewPattern => 'Pattern';

  @override
  String get view3d => '3D';

  @override
  String get hintPolygon =>
      'Polygon: click the corners; click the first one or press Enter to close · Esc cancels · V to stop';

  @override
  String get hintCurve =>
      'Curve: click for a corner, drag for a smooth point; click the first point or press Enter to close · Esc cancels · V to stop';

  @override
  String get hintBend =>
      'Bend: click an edge, then drag its handles · V to stop';

  @override
  String get hintStraighten => 'Straighten: click a curved edge · V to stop';

  @override
  String get hintAddPoint =>
      'Add point: click an edge where the point goes · V to stop';

  @override
  String get hintCut =>
      'Cut: click across a piece, drag for a curve, from outside to outside; Enter cuts · Esc cancels · V to stop';

  @override
  String get hintNotch =>
      'Notch: click an edge where the notch goes · V to stop';

  @override
  String get toastPieceAdded => 'Piece added.';

  @override
  String get toastNotchAdded => 'Notch added.';

  @override
  String get toastCut => 'Cut in two pieces, sewn together.';

  @override
  String get toastPointAdded => 'Point added.';

  @override
  String get distance => 'Distance';

  @override
  String get pointPicked =>
      'Corner picked. ⌘-click another corner to measure the distance between them.';

  @override
  String get pointsPicked =>
      'Between the two corners. The second one moves along the line.';

  @override
  String get straighten => 'Straighten';

  @override
  String get hintSewFirst =>
      'Sew: click an edge or a line, or drag along it to sew part of it · ⌘-click edges that follow each other to sew them as one side · V to stop';

  @override
  String get hintSewSecond =>
      'Sew: now the side to sew it to: click it, drag along part of it or ⌘-click several edges · Esc starts again';

  @override
  String get hintSewRun =>
      'Sew: ⌘-click the next edges, then click the last one or press Enter · Esc starts again';

  @override
  String get hintDraw =>
      'Draw: drag on the 3D garment; the line goes on the pattern · V to stop';

  @override
  String get hintIdle =>
      'Pattern: drag a corner or an edge, click an edge for its length · 3D: drag to pull the cloth · ⇧-drag a hem or a seam · ⌘-drag to move a piece · ⌥-click to pin';

  @override
  String get hintSelection =>
      'Adjust the selection in the inspector · ⌫ deletes · ⌘Z undoes';

  @override
  String statsSim(int points, String backend, String stepMs, String time) {
    return 'Cloth $points points · $backend step $stepMs ms · $time s · ';
  }

  @override
  String statsRender(String boardMs, String garmentMs, String fps) {
    return 'Rust render · pattern $boardMs ms · 3D $garmentMs ms · $fps fps';
  }

  @override
  String get toastSewn => 'Sewn. Flip it in the inspector if it looks twisted.';

  @override
  String get toastPinned => 'Pinned. Remove it in the inspector.';

  @override
  String get toastLineAdded => 'Line added to the pattern.';

  @override
  String deleteTitle(String name) {
    return 'Delete “$name”?';
  }

  @override
  String get deleteUndo => 'You can undo this with ⌘Z.';

  @override
  String get camFront => 'Front';

  @override
  String get camSide => 'Side';

  @override
  String get camBack => 'Back';

  @override
  String get camFrontTip => 'View from the front  1';

  @override
  String get camSideTip => 'View from the side  2';

  @override
  String get camBackTip => 'View from the back  3';

  @override
  String get camThreeQuarterTip => 'View from the three-quarter angle  4';

  @override
  String get simulate => 'Simulate';

  @override
  String get pause => 'Pause';

  @override
  String get simulateTip => 'Drape the garment  Space';

  @override
  String get pauseTip => 'Pause  Space';

  @override
  String get restartTip => 'Start again from the pattern';

  @override
  String get motionTip => 'Make the avatar move';

  @override
  String get motionStill => 'Stand still';

  @override
  String get motionWalk => 'Walk';

  @override
  String get motionIdle => 'Stand at ease';

  @override
  String get motionTurn => 'Turn';

  @override
  String get motionRaiseArms => 'Raise arms';

  @override
  String get maximizeViewTip => 'Use the space of both views  M';

  @override
  String get restoreViewsTip => 'Show both views again  M';

  @override
  String get inspector => 'Inspector';

  @override
  String get selectPanel => 'Select a panel';

  @override
  String selectPanelHint(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pieces',
      one: '1 piece',
    );
    return 'Click a piece on the pattern or the garment\nto adjust it here. $_temp0 in this project.';
  }

  @override
  String get arrange => 'Arrange around the body';

  @override
  String get arrangeHint =>
      'Places every piece around the body in seam order, at the waist.';

  @override
  String get pins => 'Pins';

  @override
  String get pinsEmpty => '⌥-click the cloth in 3D to hold it in place.';

  @override
  String get pinsAdd => '⌥-click the cloth in 3D to add one.';

  @override
  String get removePin => 'Remove this pin';

  @override
  String get wholeGarment => 'Whole garment';

  @override
  String piecesSelected(int count) {
    return '$count pieces selected. Click one piece on the pattern to edit it.';
  }

  @override
  String get piece => 'Piece';

  @override
  String get renamePiece => 'Rename the piece (or double-click its name)';

  @override
  String get renamePieceOk => 'Rename (Enter)';

  @override
  String get mirrorCopy => 'Mirror copy — follows its source';

  @override
  String get unlink => 'Unlink';

  @override
  String get area => 'Area';

  @override
  String get perimeter => 'Perimeter';

  @override
  String get seamAllowance => 'Seam allowance';

  @override
  String get grainline => 'Grainline';

  @override
  String get mirror => 'Mirror';

  @override
  String get onTheBody => 'On the body';

  @override
  String get notSimulated => 'Not simulated yet: place it to see it in 3D.';

  @override
  String get placeOnBody => 'Place on the body';

  @override
  String get aroundBody => 'Body';

  @override
  String get aroundLeftArm => 'Left arm';

  @override
  String get aroundRightArm => 'Right arm';

  @override
  String get aroundLeftLeg => 'Left leg';

  @override
  String get aroundRightLeg => 'Right leg';

  @override
  String get aroundFlat => 'Flat';

  @override
  String get aroundAngle => 'Around (0 = front)';

  @override
  String get aroundArmAngle => 'Around the arm (0 = outside)';

  @override
  String get topHeight => 'Top height';

  @override
  String get fabric => 'Fabric';

  @override
  String get garments => 'Garments';

  @override
  String get garmentsHint =>
      'Worn from the body out: a garment drapes over the ones above it in the list.';

  @override
  String get garmentNextToBody => 'Next to the body';

  @override
  String get garmentOver => 'Worn over the one above';

  @override
  String get garmentMoveIn => 'Wear it under';

  @override
  String get garmentMoveOut => 'Wear it over';

  @override
  String get garmentTakeOut => 'Take it out of the drape';

  @override
  String get garmentPutBack => 'Put it back in the drape';

  @override
  String get garmentOutOfDrape => 'Out of the drape';

  @override
  String get garmentRemove => 'Delete the garment and its pieces';

  @override
  String get addGarment => 'Add a garment';

  @override
  String garmentNewName(int number) {
    return 'Garment $number';
  }

  @override
  String get pieceGarment => 'Garment';

  @override
  String get fabricCotton => 'Cotton';

  @override
  String get fabricCottonBlurb => 'Crisp and breathable';

  @override
  String get fabricDenim => 'Denim';

  @override
  String get fabricDenimBlurb => 'Heavy, holds its shape';

  @override
  String get fabricJersey => 'Jersey';

  @override
  String get fabricJerseyBlurb => 'Stretchy knit';

  @override
  String get fabricSilk => 'Silk';

  @override
  String get fabricSilkBlurb => 'Light and fluid';

  @override
  String get fabricWool => 'Wool';

  @override
  String get fabricWoolBlurb => 'Soft, with body';

  @override
  String edgeLength(String length) {
    return 'Edge · $length cm';
  }

  @override
  String get edgeOpen =>
      'Open edge (a hem or an opening). In 3D, ⇧-drag it to lengthen the garment.';

  @override
  String get edgeLengthLabel => 'Length';

  @override
  String edgeSewn(String piece, String mismatch) {
    return 'Sewn to $piece · $mismatch mm difference. Its length changes on both sides. In 3D, ⇧-drag it to take it in.';
  }

  @override
  String get matchOtherSide => 'Make the other side this length';

  @override
  String get gathered => 'Gathered';

  @override
  String gatheredHint(String ratio) {
    return 'The longer side is gathered evenly onto the shorter one ($ratio : 1).';
  }

  @override
  String get flipSeam => 'Flip seam';

  @override
  String get unpick => 'Unpick';

  @override
  String changeTakenIn(String amount) {
    return 'Taken in $amount mm';
  }

  @override
  String changeLetOut(String amount) {
    return 'Let out $amount mm';
  }

  @override
  String changeLonger(String amount) {
    return 'Longer $amount mm';
  }

  @override
  String changeMoved(String amount) {
    return 'Moved $amount mm';
  }

  @override
  String changeShorter(String amount) {
    return 'Shorter $amount mm';
  }

  @override
  String get learnIntro =>
      'Short how-tos. “Show me” points at where it happens; your project is not changed.';

  @override
  String get showMe => 'Show me';

  @override
  String get learnDrape => 'Drape the garment';

  @override
  String get learnDrapeHow =>
      'Press **Simulate** (**Space**): the pieces close their seams and fall onto the body.';

  @override
  String get learnFabric => 'Try another fabric';

  @override
  String get learnFabricHow =>
      'With nothing selected, pick a swatch under **Fabric**. Fabric changes how the garment hangs.';

  @override
  String get learnLength => 'Change a length';

  @override
  String get learnLengthHow =>
      'Click an edge (a side seam to lengthen, a hem to widen), then type its **Length** or drag the slider. Or hold **⇧** and drag the hem on the garment.';

  @override
  String get learnFit => 'Take in a seam';

  @override
  String get learnFitHow =>
      'Hold **⇧** and drag a seam on the garment: towards the body takes it in, away lets it out.';

  @override
  String get learnSew => 'Sew two pieces';

  @override
  String get learnSewHow =>
      'With **Sew** (**S**), click an edge on the pattern, then the edge to sew it to.';

  @override
  String get learnDraw => 'Draw a style line';

  @override
  String get learnDrawHow =>
      'With **Draw on garment** (**D**), drag across the 3D garment: the line lands on the pattern.';

  @override
  String get learnMove => 'Move a piece around the body';

  @override
  String get learnMoveHow =>
      'Hold **⌘** and drag a piece on the garment, or set it in the inspector.';

  @override
  String get learnPin => 'Pull and pin the cloth';

  @override
  String get learnPinHow =>
      'Drag the cloth in 3D to pull it. **⌥-click** pins it where it is.';

  @override
  String get learnLook => 'Look around';

  @override
  String get learnLookHow =>
      'Drag to turn around the body; scroll or pinch to zoom. Keys **1**–**4**: front, side, back, three quarters.';

  @override
  String get learnBody => 'Dress another body';

  @override
  String get learnBodyHow =>
      'Open **Body**: choose a size or enter measurements, and the garment drapes on it.';

  @override
  String get learnDrapeStop => 'Simulate';

  @override
  String get learnDrapeStopText =>
      'Press **Simulate** (**Space**) to drape the garment, again to pause. The arrow next to it starts again from the pattern.';

  @override
  String get learnFabricStop => 'Fabric';

  @override
  String get learnFabricStopText =>
      'With nothing selected, the panel on the right shows the garment: pick a swatch under **Fabric**. The drape follows.';

  @override
  String get learnPickEdge => 'Pick an edge';

  @override
  String get learnPickEdgeText =>
      'Click an edge on the pattern, or on the garment.';

  @override
  String get learnEdgeLength => 'Its length';

  @override
  String get learnEdgeLengthText =>
      'Type the new **Length** and press **Enter**, or drag the slider. A sewn edge’s other side follows, so seams stay matched.';

  @override
  String get learnDragHem => 'Or drag it';

  @override
  String get learnDragHemText =>
      'Hold **⇧** and drag the hem on the garment: down lengthens, up shortens.';

  @override
  String get learnDragSeam => 'Or drag it';

  @override
  String get learnDragSeamText =>
      'Hold **⇧** and drag a seam on the garment: towards the body takes it in, away lets it out. Near one end, only that end moves.';

  @override
  String get learnSewTool => 'Sew tool';

  @override
  String get learnSewToolText => 'Pick **Sew** here, or press **S**.';

  @override
  String get learnTwoEdges => 'Click two edges';

  @override
  String get learnTwoEdgesText =>
      'Click an edge, then the edge to sew it to. Matching arrows show how they join.';

  @override
  String get learnDrawTool => 'Draw on garment';

  @override
  String get learnDrawToolText =>
      'Pick **Draw on garment** here, or press **D**.';

  @override
  String get learnDrawAcross => 'Drag across the garment';

  @override
  String get learnDrawAcrossText =>
      'The line lands on the pattern, for a yoke or a pocket.';

  @override
  String get learnDragPiece => 'Drag a piece';

  @override
  String get learnDragPieceText =>
      'Hold **⌘** and drag a piece: sideways turns it around the body, up and down changes its height.';

  @override
  String get learnSetPlacement => 'Or set it here';

  @override
  String get learnSetPlacementText =>
      'Select the piece: **On the body** has the same settings, as numbers you can type.';

  @override
  String get learnPullPin => 'Pull and pin';

  @override
  String get learnPullPinText =>
      'Drag the cloth to pull it. **⌥-click** pins it where it is.';

  @override
  String get learnYourPins => 'Your pins';

  @override
  String get learnYourPinsText =>
      'With nothing selected, pins are listed under **Pins**; **×** removes one.';

  @override
  String get learnViews => 'Views';

  @override
  String get learnViewsText =>
      '**Front**, **Side**, **Back** and **¾**, or the keys **1**–**4**.';

  @override
  String get learnTurnZoom => 'Turn and zoom';

  @override
  String get learnTurnZoomText =>
      'Drag around the body to turn; scroll, or pinch on a trackpad, to zoom.';

  @override
  String get learnBodyStop => 'Body';

  @override
  String get learnBodyStopText =>
      'Choose a size or enter measurements: the body is refitted and the garment drapes on it.';

  @override
  String errNotFound(String id) {
    return '$id does not exist.';
  }

  @override
  String errLinkedMirror(String panel) {
    return '$panel is a linked mirror copy: edit its source, or unlink it first.';
  }

  @override
  String get errSewn =>
      'This edge is sewn or zipped: unpick the seam or remove the zipper first.';

  @override
  String get errNothingToUndo => 'Nothing to undo.';

  @override
  String get errNothingToRedo => 'Nothing to redo.';

  @override
  String get errNothingToSimulate =>
      'Nothing to simulate: place at least one piece on the body.';

  @override
  String get errInverted => 'That would turn a piece inside out.';

  @override
  String errRange(String min, String max) {
    return 'The value must be between $min and $max.';
  }

  @override
  String get errNotPositive => 'The value must be greater than zero.';

  @override
  String get errNegative => 'The value must not be negative.';

  @override
  String get errEmpty => 'The name must not be empty.';

  @override
  String get errNotSimple => 'The outline must not cross itself.';

  @override
  String get errNotAcross =>
      'The cut must cross one piece from side to side: start and end outside it.';

  @override
  String get errMirrored =>
      'This piece has a linked mirror copy: unlink the copy first.';

  @override
  String get errLastGarment => 'A project keeps at least one garment.';

  @override
  String get errFileIo => 'Could not read or write the project files.';

  @override
  String errImportMalformed(String format) {
    return 'This file cannot be read as $format.';
  }

  @override
  String get errImportBinaryDxf =>
      'Binary DXF files cannot be read: save the file as ASCII DXF.';

  @override
  String get errImportNoPieces =>
      'No pattern piece found: the drawing has no closed outline.';

  @override
  String get errImportPagesDiffer =>
      'The pages of this PDF do not repeat one drawing, so they cannot be put back together.';

  @override
  String get errImportEncrypted => 'This PDF is protected and cannot be read.';

  @override
  String get errReadFile => 'Could not read this file.';

  @override
  String get errNotAProject => 'This folder is not a Rustle3D project.';

  @override
  String get errTooNew =>
      'This project was made by a newer version of Rustle3D.';

  @override
  String get errCorrupted => 'This project file is damaged.';

  @override
  String get errAlreadyExists => 'A project already exists there.';

  @override
  String get errArchiveInvalid => 'This file is not a readable .zip archive.';

  @override
  String get errArchiveNoProject => 'This archive holds no Rustle3D project.';

  @override
  String errArchiveTooLarge(String limit) {
    return 'This archive is too large to import (more than $limit MB once unpacked).';
  }

  @override
  String get photo => 'Photo';

  @override
  String get photoTip => 'Set up a photo of the garment and render it';

  @override
  String get photoTitle => 'Photo scene';

  @override
  String get photoIntro =>
      'The 3D view shows the photo as it will be. Drag in it to turn around the model, scroll to come closer.';

  @override
  String get photoFocal => 'Lens';

  @override
  String get photoPlaceSize => 'Place size';

  @override
  String get photoSceneHint =>
      'A short lens shows more of the place around, a long one less, with a flatter look. With a place, its size is how far its walls stand from the model: about 3 m for a room, more outdoors.';

  @override
  String get photoDefaults => 'Default settings';

  @override
  String get photoDefaultsTip =>
      'Put the turn, brightness, place size and lens back as they were at first';

  @override
  String get photoSceneCloseTip =>
      'Close the scene (it is saved with the project)';

  @override
  String get photoQuality => 'Quality';

  @override
  String get photoQuick => 'Quick';

  @override
  String get photoQuickHint => 'A first look in seconds';

  @override
  String get photoGood => 'Good';

  @override
  String get photoGoodHint => 'Clean enough to show';

  @override
  String get photoBest => 'Best';

  @override
  String get photoBestHint => 'For print and close-ups';

  @override
  String get photoLighting => 'Lighting';

  @override
  String get lightStudio => 'Studio';

  @override
  String get lightStudioHint =>
      'Key, fill and rim lights on a light grey backdrop';

  @override
  String get lightSoft => 'Soft';

  @override
  String get lightSoftHint => 'Large soft lights on a white backdrop';

  @override
  String get lightDramatic => 'Dramatic';

  @override
  String get lightDramaticHint => 'One hard light on a dark backdrop';

  @override
  String get lightEnvironment => 'Place';

  @override
  String get lightEnvironmentHint =>
      'A real place (an HDR image) lights the garment and shows behind it, with its shadow on the floor';

  @override
  String get photoEnvironmentAdd => 'Add .hdr…';

  @override
  String get photoEnvironmentFiles => 'HDR images';

  @override
  String get photoTurn => 'Turn';

  @override
  String get photoBrightness => 'Brightness';

  @override
  String get photoSize => 'Size';

  @override
  String get photoSizeView => 'As the 3D view';

  @override
  String get photoAdvanced => 'Advanced';

  @override
  String get photoSamples => 'Samples per pixel';

  @override
  String get photoBounces => 'Light bounces';

  @override
  String get photoDenoise => 'Remove noise';

  @override
  String get photoRender => 'Render';

  @override
  String photoRendering(int done, int total, String seconds) {
    return 'Rendering… $done / $total samples · $seconds s';
  }

  @override
  String photoDone(String seconds) {
    return 'Done in $seconds s';
  }

  @override
  String photoStopped(int done) {
    return 'Stopped at $done samples';
  }

  @override
  String photoFailed(String reason) {
    return 'The photo could not be rendered: $reason';
  }

  @override
  String get photoStop => 'Stop';

  @override
  String get photoStopTip => 'Stop refining and keep the image as it is';

  @override
  String get photoSave => 'Save image…';

  @override
  String get photoClose => 'Close';

  @override
  String get photoCloseTip => 'Put the photo away';

  @override
  String photoSaved(String file) {
    return 'Saved $file.';
  }

  @override
  String get errPhotoTooLarge =>
      'This image is too large. Choose a smaller size (at most 3840 × 2160).';

  @override
  String get errNoPhoto => 'There is no photo to save.';

  @override
  String get learnPhoto => 'Take a photo';

  @override
  String get learnPhotoHow =>
      'A photorealistic image of the garment, to show a client or publish.';

  @override
  String get learnPhotoStop => 'Photo';

  @override
  String get learnPhotoStopText =>
      'Turn the 3D view to the angle you like, click the **camera** button, choose the quality and the lighting, then **Render**. The image gets cleaner in the 3D view; **Save image…** keeps it as a PNG file.';

  @override
  String get sizes => 'Sizes';

  @override
  String get sizesHint =>
      'Grade the pattern to a range of sizes. It stays drawn in the base size; the other sizes\' outlines nest around each piece.';

  @override
  String get addSizes => 'Add a size range';

  @override
  String get sizeChart => 'Size chart';

  @override
  String get chartWomen => 'Women';

  @override
  String get chartMen => 'Men';

  @override
  String get sizeFrom => 'From';

  @override
  String get sizeTo => 'To';

  @override
  String get sizeBase => 'Base size';

  @override
  String get sizesDialogHint =>
      'Pieces added from a template afterwards are graded to every size. Each size is draped on a body with its measurements.';

  @override
  String get sizeShown => 'Shown in 3D';

  @override
  String get sizesChange => 'Change';

  @override
  String get sizesRemove => 'Remove the sizes';

  @override
  String get sizeBaseTag => 'base';

  @override
  String get gradeStep => 'Grade per size';

  @override
  String get gradeStepHint =>
      'How far this point moves from one size to the next.';

  @override
  String get gradeCustom =>
      'Set per size (by a template or an agent). A step typed here replaces it.';

  @override
  String get errNoSizes => 'Add a size range first.';

  @override
  String get errUnknownSize => 'That size is not in the range.';

  @override
  String get errTooFewSizes => 'A size range needs at least two sizes.';

  @override
  String get errDuplicateName => 'Two sizes have the same name.';

  @override
  String get errBodyOutOfRange =>
      'A size of this range would give the avatar measurements outside what it accepts.';

  @override
  String get errBaseOffset =>
      'The base size cannot move: it is the pattern as drawn.';

  @override
  String get sizesOnBoard => 'Show the sizes on the board';

  @override
  String stepName(String step) {
    String _temp0 = intl.Intl.selectLogic(step, {
      'pattern_add': 'Add pattern',
      'project_rename': 'Rename project',
      'project_set_display_unit': 'Change units',
      'avatar_set': 'Change body',
      'avatar_remove': 'Remove body',
      'panel_create_rect': 'Add rectangle',
      'panel_create_polygon': 'Add polygon',
      'panel_create_path': 'Add curved piece',
      'panel_rename': 'Rename piece',
      'panel_translate': 'Move piece',
      'panel_rotate': 'Rotate piece',
      'panel_scale': 'Scale piece',
      'panel_set_grain': 'Set grainline',
      'panel_set_seam_allowance': 'Set seam allowance',
      'panel_mirror': 'Mirror piece',
      'panel_unlink': 'Unlink copy',
      'panel_place': 'Place piece',
      'panel_delete': 'Delete piece',
      'panel_cut': 'Cut piece',
      'panel_paste': 'Paste',
      'point_move': 'Move point',
      'point_set_distance': 'Set distance',
      'point_remove': 'Remove point',
      'edge_set_curve': 'Bend edge',
      'edge_split': 'Add point',
      'edge_move': 'Move edge',
      'notch_add': 'Add notch',
      'notch_move': 'Move notch',
      'notch_remove': 'Remove notch',
      'internal_line_add': 'Draw line',
      'internal_line_remove': 'Remove line',
      'seam_add': 'Sew',
      'seam_remove': 'Unsew',
      'seam_flip': 'Flip seam',
      'seam_set_gathered': 'Gather seam',
      'edge_lengthen': 'Lengthen',
      'edge_set_length': 'Set length',
      'seam_take_in': 'Take in',
      'seam_match_length': 'Match lengths',
      'pin_add': 'Pin',
      'pin_remove': 'Unpin',
      'garment_set_fabric': 'Change fabric',
      'garment_arrange': 'Arrange pieces',
      'garment_set_color': 'Change colour',
      'garment_add': 'Add garment',
      'garment_rename': 'Rename garment',
      'garment_remove': 'Remove garment',
      'garment_set_layer': 'Change layer',
      'panel_set_garment': 'Move to garment',
      'grading_set_sizes': 'Set sizes',
      'grading_use_chart': 'Use size chart',
      'grading_clear': 'Remove sizes',
      'grading_show_size': 'Show size',
      'point_set_grade': 'Grade point',
      'template_apply': 'Add template',
      'pattern_import': 'Import pattern',
      'batch': 'Several changes',
      'constraint_add': 'Add constraint',
      'constraint_remove': 'Remove constraint',
      'constraint_set_value': 'Change dimension',
      'point_set_smooth': 'Smooth point',
      'panel_unfold': 'Unfold piece',
      'guide_add': 'Add construction line',
      'guide_move': 'Move construction line',
      'guide_remove': 'Remove construction line',
      'panel_create_ellipse': 'Add ellipse',
      'panel_create_sector': 'Add circle skirt',
      'edge_set_arc': 'Make arc',
      'constraint_set_formula': 'Set formula',
      'variable_set': 'Set variable',
      'variable_remove': 'Remove variable',
      'seam_set_ease': 'Set ease',
      'dart_add': 'Add dart',
      'pleat_add': 'Add pleat',
      'panel_add_hole': 'Cut hole',
      'panel_remove_hole': 'Fill hole',
      'panel_boolean': 'Join or cut pieces',
      'object_set': 'Make an object',
      'object_clear': 'Wear on a body',
      'other': '$step',
    });
    return '$_temp0';
  }

  @override
  String undoStep(String name) {
    return 'Undo $name  ⌘Z';
  }

  @override
  String redoStep(String name) {
    return 'Redo $name  ⇧⌘Z';
  }

  @override
  String get nothingToUndo => 'Nothing to undo';

  @override
  String get nothingToRedo => 'Nothing to redo';

  @override
  String get historyTip => 'History: go back or forward to any step';

  @override
  String get historyTitle => 'History';

  @override
  String get historyStart => 'Opened';

  @override
  String get historyEmpty => 'No changes yet.';

  @override
  String copied(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pieces copied',
      one: '1 piece copied',
    );
    return '$_temp0, with the seams between them. ⌘V pastes them, here or in another project.';
  }

  @override
  String pasted(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pieces pasted',
      one: '1 piece pasted',
    );
    return '$_temp0.';
  }

  @override
  String get nothingToPaste => 'Nothing to paste: copy pieces first (⌘C).';

  @override
  String get selectToCopy => 'Select pieces to copy.';

  @override
  String get errCancelled => 'Stopped: nothing was changed.';

  @override
  String get busyStop => 'Stop';

  @override
  String get busyStep => 'Working…';

  @override
  String busySizes(String size, int done, int total) {
    return 'Drafting size $size ($done of $total)';
  }

  @override
  String busySizesStart(int total) {
    return 'Drafting every size ($total)';
  }

  @override
  String get busyBodyFit => 'Fitting the body to the measurements…';

  @override
  String get busyDrape => 'Placing the pieces around the body…';

  @override
  String get busyExport => 'Exporting…';

  @override
  String get busyImport => 'Reading the pattern…';

  @override
  String get busyOpening => 'Opening the project';

  @override
  String get busyTemplate => 'Adding the template';

  @override
  String get busySizesTitle => 'Setting the sizes';

  @override
  String get busyBodyTitle => 'Changing the body';

  @override
  String get busyDrapeTitle => 'Starting the drape';

  @override
  String get bodyFitting => 'Fitting the body…';

  @override
  String get toolShapes => 'Shapes';

  @override
  String get toolEllipse => 'Ellipse';

  @override
  String get toolEllipseHint => 'A circle or an ellipse of the size you type';

  @override
  String get toolSector => 'Circle skirt';

  @override
  String get toolSectorHint =>
      'Ring pieces from the waist and the skirt\'s length';

  @override
  String get toolArc => 'Arc';

  @override
  String get toolArcHint => 'Click an edge to make it a circular arc';

  @override
  String get toolGuide => 'Construction line';

  @override
  String get toolGuideHint =>
      'Click two points: a line to draft against, never cut or sewn';

  @override
  String get hintGuide =>
      'Construction line: click two points · Esc cancels · V to stop';

  @override
  String get toolDimension => 'Dimension';

  @override
  String get toolDimensionHint =>
      'Click an edge, two corners or two edges, then where the dimension goes; type its value or a formula';

  @override
  String get hintDimension =>
      'Dimension: click an edge (its length), a corner then another (the distance) or two edges (the angle) · V to stop';

  @override
  String get hintDimensionPlace =>
      'Dimension: click where it goes (between two corners, pull it above or beside them for a width or a height), then type its value · Esc cancels';

  @override
  String get hintArc => 'Arc: click an edge · V to stop';

  @override
  String get toastGuideAdded => 'Construction line added.';

  @override
  String get symmetricHalf => 'Half piece';

  @override
  String get symmetricHalfTip =>
      'Draw half of a symmetric piece: the closing edge is the axis, the other half follows';

  @override
  String get width => 'Width';

  @override
  String get height => 'Height';

  @override
  String get sectorWaist => 'Waist';

  @override
  String get sectorLength => 'Skirt length';

  @override
  String get sectorFull => 'Full';

  @override
  String get sectorHalf => 'Half';

  @override
  String get sectorQuarter => 'Quarter';

  @override
  String get sectorHint =>
      'A full circle skirt is two half pieces, to sew at the sides.';

  @override
  String get arcTitle => 'Arc';

  @override
  String get arcBulge => 'Bulge';

  @override
  String get arcHint =>
      'How far the middle of the arc is from the straight line; negative curves inward.';

  @override
  String get constraints => 'Constraints';

  @override
  String get constraintsEmpty =>
      'Pick a corner or an edge on the board (⌘-click a second one) to constrain it.';

  @override
  String constraintsFreedom(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count degrees of freedom left',
      one: '1 degree of freedom left',
      zero: 'Fully constrained',
    );
    return '$_temp0';
  }

  @override
  String get constraintsRedundant => 'Some constraints repeat others.';

  @override
  String get constraintBroken => 'Does not hold';

  @override
  String constraintKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'fixed': 'Fixed',
      'coincident': 'Joined',
      'horizontal': 'Horizontal',
      'vertical': 'Vertical',
      'parallel': 'Parallel',
      'perpendicular': 'Square',
      'tangent': 'Smooth',
      'equal_length': 'Same length',
      'on_edge': 'On the edge',
      'distance': 'Distance',
      'width': 'Width',
      'height': 'Height',
      'length': 'Length',
      'angle': 'Angle',
      'on_guide': 'On the line',
      'notch_distance': 'Notch distance',
      'guide_distance': 'Distance to the line',
      'other': '$kind',
    });
    return '$_temp0';
  }

  @override
  String get barHint =>
      'Pick corners, edges, notches or construction lines on the board (⇧- or ⌘-click adds one) to constrain them.';

  @override
  String barCorners(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count corners',
      one: '1 corner',
    );
    return '$_temp0';
  }

  @override
  String barEdges(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count edges',
      one: '1 edge',
    );
    return '$_temp0';
  }

  @override
  String barNotches(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count notches',
      one: '1 notch',
    );
    return '$_temp0';
  }

  @override
  String barLines(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count lines',
      one: '1 line',
    );
    return '$_temp0';
  }

  @override
  String get barFixedHint =>
      'The corner stays where it is when the drawing is solved.';

  @override
  String get barSmoothHint =>
      'No corner: the outline passes smoothly through the point.';

  @override
  String get barCornerHint =>
      'A corner again: the two edges no longer line up.';

  @override
  String get barRemovePointHint =>
      'Remove the corner; its two edges become one.';

  @override
  String get barWidthHint =>
      'Dimension the distance across between the two corners, then type it.';

  @override
  String get barHeightHint =>
      'Dimension the distance up and down between the two corners, then type it.';

  @override
  String get barDistanceHint =>
      'Dimension the straight distance between the two corners, then type it.';

  @override
  String get barJoinedHint =>
      'The two corners at the same place (on two pieces, usually).';

  @override
  String get barLengthHint =>
      'Dimension the edge\'s length along its curve, then type it.';

  @override
  String get barHorizontalHint => 'The edge\'s ends at the same height.';

  @override
  String get barVerticalHint => 'The edge\'s ends one above the other.';

  @override
  String get barAngleHint =>
      'Dimension the angle between the two edges, then type it.';

  @override
  String get barSameLengthHint => 'The edges as long as the first one picked.';

  @override
  String get barParallelHint => 'The edges parallel to the first one picked.';

  @override
  String get barSquareHint => 'The two edges at right angles.';

  @override
  String get barOnEdgeHint =>
      'The corner on the edge (on its line, extended, for a straight edge).';

  @override
  String get barNotchStart => 'From the start';

  @override
  String get barNotchStartHint =>
      'Dimension the notch\'s distance along its edge from the edge\'s start; it keeps it when the edge changes.';

  @override
  String get barNotchEnd => 'From the end';

  @override
  String get barNotchEndHint =>
      'Dimension the notch\'s distance along its edge from the edge\'s end; it keeps it when the edge changes.';

  @override
  String get barRemoveNotch => 'Remove notch';

  @override
  String get barRemoveNotchHint => 'Remove the notch.';

  @override
  String get barGuideDistanceHint =>
      'Dimension the corner\'s distance from the line, then type it; moving the line moves the corner.';

  @override
  String get barOnGuideHint =>
      'The corner on the line; moving the line moves the corner.';

  @override
  String get barLevel => 'Level';

  @override
  String get barLevelHint =>
      'Turn the line about its first end so that it is level.';

  @override
  String get barUpright => 'Upright';

  @override
  String get barUprightHint =>
      'Turn the line about its first end so that it is upright.';

  @override
  String get barRemoveLine => 'Remove line';

  @override
  String get barRemoveLineHint =>
      'Remove the construction line (and its constraints).';

  @override
  String get barRotateHint =>
      'Turn the piece by an angle you type. Its frame\'s round handle turns it by hand.';

  @override
  String get barScaleHint =>
      'Make the piece wider or taller by a percentage. Its frame\'s corners scale it by hand.';

  @override
  String get barMirrorHint => 'Add a mirrored copy that follows this piece.';

  @override
  String get barJoinHint =>
      'Merge another piece into this one: pick it in the list.';

  @override
  String get barCutOutHint =>
      'Cut another piece\'s shape out of this one: pick it in the list.';

  @override
  String get barDeleteHint => 'Delete the piece.';

  @override
  String get barNothing => 'Nothing picked';

  @override
  String get barShortRotate => 'Rotate';

  @override
  String get barShortScale => 'Scale';

  @override
  String get barShortFromStart => 'From start';

  @override
  String get barShortFromEnd => 'From end';

  @override
  String get barDeleteAllHint => 'Delete the selected pieces.';

  @override
  String get dimensionRename => 'Name: formulas can use this dimension';

  @override
  String get dimensionNameTitle => 'Name of the dimension';

  @override
  String get dimensionNameHelp =>
      'Letters, digits and _, starting with a letter. Formulas then use it like a variable: a sleeve cap\'s length can be \"armhole + 1.5 cm\". Empty: no name.';

  @override
  String get constrain => 'Constrain';

  @override
  String get removeConstraint => 'Remove';

  @override
  String get pointSmooth => 'Smooth';

  @override
  String get pointCorner => 'Corner';

  @override
  String get pointRemove => 'Remove point';

  @override
  String get symmetricPiece => 'Symmetric piece';

  @override
  String get unlinkHalves => 'Separate halves';

  @override
  String get unfold => 'Unfold here';

  @override
  String get makeArc => 'Arc';

  @override
  String snapKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'point': 'Point',
      'midpoint': 'Middle',
      'crossing': 'Crossing',
      'edge': 'On edge',
      'guide': 'On line',
      'axis': 'On axis',
      'horizontal': 'Level',
      'vertical': 'Aligned',
      'other': '$kind',
    });
    return '$_temp0';
  }

  @override
  String get errDerived =>
      'This part follows the drawn half or the original piece: edit that one.';

  @override
  String get errUnsolvable =>
      'These constraints cannot all hold: nothing was changed.';

  @override
  String get errSymmetric => 'A symmetric piece: separate its halves first.';

  @override
  String get errNotStraight => 'This needs a straight edge.';

  @override
  String get errDuplicate => 'Already constrained this way.';

  @override
  String get formulaTip =>
      'Formula: compute this value from the body\'s measurements';

  @override
  String get formulaTitle => 'Formula';

  @override
  String get formulaHelp =>
      'Measurements: height, bust, waist, hips. Units: mm, cm, in. + − × ÷, min, max. Your variables too. Empty: the value stays as it is.';

  @override
  String get variables => 'Variables';

  @override
  String get variablesHint =>
      'Names for formulas, like ease = 2 cm. Pieces with formulas redraw themselves for a new body and in every size.';

  @override
  String get variable => 'Variable';

  @override
  String get variableName => 'Name';

  @override
  String get variableAdd => 'Add a variable';

  @override
  String get errGradedByFormula =>
      'This point is graded by formulas: change them instead.';

  @override
  String errFormulaSyntax(String at) {
    return 'The formula cannot be read at character $at.';
  }

  @override
  String errFormulaUnknown(String name) {
    return '“$name” is neither a measurement nor a variable.';
  }

  @override
  String errFormulaNoBody(String name) {
    return '“$name” needs the body’s measurements: set them first.';
  }

  @override
  String errFormulaCycle(String name) {
    return '“$name” is computed from itself.';
  }

  @override
  String get errFormulaUnit =>
      'The units do not match (a length needs a unit, like 10 mm).';

  @override
  String get errFormulaDomain =>
      'The formula has no value (a division by zero?).';

  @override
  String errFormulaRange(String value) {
    return 'The formula gives $value: too small for this dimension.';
  }

  @override
  String errFormulaBadName(String name) {
    return '“$name” cannot name a variable: a letter, then letters, digits or _.';
  }

  @override
  String get toolDetails => 'Darts and pleats';

  @override
  String get toolDart => 'Dart';

  @override
  String get toolDartHint =>
      'Click an edge where the dart goes: drawn, folded and sewn';

  @override
  String get toolPleat => 'Pleat';

  @override
  String get toolPleatHint =>
      'Click an edge where the pleat starts: folded and sewn';

  @override
  String get toolHole => 'Hole';

  @override
  String get toolHoleHint =>
      'Select a piece, then click the corners of a hole inside it';

  @override
  String get hintDart => 'Dart: click an edge where it goes · V to stop';

  @override
  String get hintPleat => 'Pleat: click an edge where it starts · V to stop';

  @override
  String get hintHole =>
      'Hole: click its corners inside the selected piece, Enter to cut it · Esc cancels';

  @override
  String get toastHoleAdded => 'Hole cut.';

  @override
  String get holeSelectFirst => 'Select the piece to cut the hole in first.';

  @override
  String get dartWidth => 'Width at the edge';

  @override
  String get dartLength => 'Length';

  @override
  String get pleatDepth => 'Depth';

  @override
  String get pleatStitched => 'Stitched down';

  @override
  String get pleatKnife => 'Knife';

  @override
  String get pleatBox => 'Box';

  @override
  String get join => 'Join';

  @override
  String get cutOut => 'Cut out';

  @override
  String get ease => 'Ease';

  @override
  String get errOverlaps => 'This stretch is already sewn.';

  @override
  String get errNotWholeEdges => 'This works on seams between two whole edges.';

  @override
  String get errApart => 'The result would be several pieces, or nothing.';

  @override
  String get errHasHoles => 'This piece has holes: fill them first.';

  @override
  String get rotatePiece => 'Rotate';

  @override
  String get rotatePieceHint =>
      'Counter-clockwise; a negative angle turns the other way.';

  @override
  String get rotateAngle => 'Angle';

  @override
  String get scalePiece => 'Scale';

  @override
  String get scaleAcross => 'Across';

  @override
  String get scaleUpDown => 'Up and down';

  @override
  String get errConstrained =>
      'This piece has dimensions or constraints that would no longer hold: remove them first.';

  @override
  String get errNotJoined =>
      'These edges do not follow each other on one piece.';

  @override
  String get close => 'Close';

  @override
  String get marker => 'Marker';

  @override
  String get exportMarker => 'Marker: the pieces laid out on the fabric…';

  @override
  String get markerHint =>
      'The pieces, with their seam allowances, laid out on the fabric along its grain. Pick the sizes to cut and the fabric\'s width: you get the length to buy and its cost.';

  @override
  String get markerGarment => 'Garment';

  @override
  String get markerSizes => 'Sizes to cut';

  @override
  String get markerWidth => 'Fabric width';

  @override
  String get markerLayOut => 'Lay out';

  @override
  String get markerLength => 'Length';

  @override
  String get markerEfficiency => 'Fabric used';

  @override
  String get markerYardage => 'To buy';

  @override
  String get markerCost => 'Cost';

  @override
  String get markerNoPrice =>
      'Give the fabric a price per metre (inspector, with nothing selected, under the garment) to see the cost.';

  @override
  String get markerSaveSvg => 'Save SVG';

  @override
  String get markerSavePdf => 'Save PDF';

  @override
  String get tpDecimal => '.';

  @override
  String get tpSizes => 'Sizes';

  @override
  String get tpSample => 'sample';

  @override
  String get tpViews => 'Views';

  @override
  String get tpFront => 'Front';

  @override
  String get tpBack => 'Back';

  @override
  String get tpColourways => 'Colourways';

  @override
  String get tpMaterials => 'Bill of materials';

  @override
  String get tpFabrics => 'Fabrics';

  @override
  String get tpGarment => 'Garment';

  @override
  String get tpFabric => 'Fabric';

  @override
  String get tpColour => 'Colour';

  @override
  String get tpWeight => 'Weight (g/m²)';

  @override
  String get tpWidth => 'Width (cm)';

  @override
  String get tpPrice => 'Price per metre';

  @override
  String get tpConsumption => 'Fabric per garment (m)';

  @override
  String get tpCost => 'Cost per garment';

  @override
  String get tpConsumptionBySize => 'Fabric per garment by size (m)';

  @override
  String get tpTrims => 'Trims';

  @override
  String get tpNoTrims => 'No trims.';

  @override
  String get tpMeasurements => 'Measurements';

  @override
  String get tpSizeChart => 'Size chart (body, cm)';

  @override
  String get tpSize => 'Size';

  @override
  String get tpHeight => 'Height';

  @override
  String get tpBust => 'Bust';

  @override
  String get tpWaist => 'Waist';

  @override
  String get tpHips => 'Hips';

  @override
  String get tpPieces => 'Pieces';

  @override
  String get tpPiece => 'Piece';

  @override
  String get tpLength => 'Length (cm)';

  @override
  String get tpPieceWidth => 'Width (cm)';

  @override
  String get tpPerimeter => 'Sewing line (cm)';

  @override
  String get tpArea => 'Area (cm²)';

  @override
  String get tpCutArea => 'Cut area (cm²)';

  @override
  String get tpAllowance => 'Allowance (mm)';

  @override
  String get tpMass => 'Mass (g)';

  @override
  String get tpPiecesBySize => 'Pieces by size: length x width (cm)';

  @override
  String get tpPattern => 'Pattern';

  @override
  String get tpSewing => 'Sewing notes';

  @override
  String get tpSeams => 'Seams';

  @override
  String get tpSideA => 'Side A';

  @override
  String get tpSideB => 'Side B';

  @override
  String get tpLengthA => 'Length A (mm)';

  @override
  String get tpLengthB => 'Length B (mm)';

  @override
  String get tpEase => 'Ease (mm)';

  @override
  String get tpGathered => 'Gathered';

  @override
  String get tpYes => 'yes';

  @override
  String get tpNoSeams => 'No seams yet.';

  @override
  String get tpNotes => 'Notes';

  @override
  String get exportTechpack => 'Tech pack (PDF)…';

  @override
  String get exportProjector => 'Projector: the pattern at 1:1…';

  @override
  String get techpackLetter => 'Letter pages';

  @override
  String get garmentNotes => 'Notes';

  @override
  String get garmentNotesHint =>
      'Sewing and finishing notes, printed in the tech pack';

  @override
  String get projectorHelp =>
      'Drag or use the arrow keys to bring the pattern under the projector (Shift: 10 cm). Home: back to the middle. M: mirror · R: turn · I: contrast · C: calibrate · Esc: close.';

  @override
  String get projectorCalibrate => 'Calibrate';

  @override
  String get projectorMirror => 'Mirror';

  @override
  String get projectorTurn => 'Turn';

  @override
  String get projectorContrast => 'Contrast';

  @override
  String get projectorCalibrateHint =>
      'The square below should be 20 cm a side on the table. Measure its sides with a ruler and type them in.';

  @override
  String get projectorMeasuredWidth => 'Width measured';

  @override
  String get projectorMeasuredHeight => 'Height measured';

  @override
  String get projectorApply => 'Apply';

  @override
  String get projectorReset => 'Reset';

  @override
  String get projectorNothing => 'No piece to project.';

  @override
  String get fabricBought => 'As bought';

  @override
  String get fabricWeight => 'Weight';

  @override
  String get fabricWidth => 'Width';

  @override
  String get fabricPrice => 'Price / m';

  @override
  String get cutArea => 'Cut area';

  @override
  String get pieceMass => 'Mass';

  @override
  String get errMarkerTooNarrow =>
      'A piece is wider than the fabric: choose a wider fabric.';

  @override
  String get errLineOutside =>
      'A line drawn on a piece now leaves it: move it back inside the piece, or delete it.';

  @override
  String garmentTotals(int pieces, String area, String mass) {
    return '$pieces pieces · $area m² of fabric cut · $mass g';
  }

  @override
  String get fitTip => 'Show where the garment is tight';

  @override
  String get fitOff => 'Fabric colours';

  @override
  String get fitStrain => 'Strain';

  @override
  String get fitGap => 'Gap to the body';

  @override
  String get fitStrainTitle => 'Strain: stretch beyond the pattern';

  @override
  String get fitGapTitle => 'Gap to the body';

  @override
  String get fitLoose => 'loose';

  @override
  String get fitTight => 'tight';

  @override
  String fitTightest(String value, String piece) {
    return 'Tightest: $value on $piece';
  }

  @override
  String get toolTape => 'Tape measure';

  @override
  String get toolTapeHint =>
      'Click on the 3D garment: its circumference there, the body\'s and the ease; or a length between two points';

  @override
  String get hintTape =>
      'Tape measure: click on the 3D garment to measure around it there, or two points for a length · drag to turn the view · V to stop';

  @override
  String get tapeHint => 'Click on the draped garment.';

  @override
  String get tapeClose => 'Put the tape away';

  @override
  String get tapeHeight => 'Height';

  @override
  String get tapeGarment => 'Garment';

  @override
  String get tapeBody => 'Body';

  @override
  String get tapeEase => 'Ease';

  @override
  String get tapeGap => 'Gap here';

  @override
  String get tapeOpen => 'The garment does not go all round here: no ease.';

  @override
  String get tapeRound => 'Round';

  @override
  String get tapeLength => 'Length';

  @override
  String get tapeFirstPoint => 'Click the first point on the draped garment.';

  @override
  String get tapeSecondPoint => 'Click the second point.';

  @override
  String get tapeAlong => 'Along the cloth';

  @override
  String get tapeTaut => 'Tape pulled taut';

  @override
  String get tapeStraight => 'Straight';

  @override
  String get tapeNotJoined =>
      'The cloth does not join these points: straight only.';

  @override
  String get printTitle => 'Print';

  @override
  String get printHint =>
      'Lies along each piece\'s grainline, at its real size.';

  @override
  String get printNone => 'No print';

  @override
  String get printStripes => 'Stripes';

  @override
  String get printGingham => 'Gingham';

  @override
  String get printDots => 'Dots';

  @override
  String get printTwill => 'Twill';

  @override
  String get printImage => 'Your image (PNG, JPEG)…';

  @override
  String get printImages => 'Images';

  @override
  String get printMissing =>
      'The image cannot be read: the plain colour is shown.';

  @override
  String get printWidth => 'Repeat width';

  @override
  String get printAngle => 'Angle to the grain';

  @override
  String get piecePrint => 'Print on this piece';

  @override
  String get piecePrintAcross => 'Shift across';

  @override
  String get piecePrintUp => 'Shift up';

  @override
  String get piecePrintTurn => 'Turn';

  @override
  String get errTextureFormat => 'This file is not a PNG or JPEG image.';

  @override
  String get errTextureBad => 'This image cannot be read.';

  @override
  String get errTextureTooLarge =>
      'This image is too large (16384 pixels a side at most).';

  @override
  String get toolTrims => 'Trims';

  @override
  String get toolButton => 'Button';

  @override
  String get toolButtonHint =>
      'Click where the button goes, then where it buttons: the end of the buttonhole, on the piece that laps over';

  @override
  String get toolZipper => 'Zipper';

  @override
  String get toolZipperHint =>
      'Click one edge near the zipper\'s bottom, then the edge it zips to';

  @override
  String get toastButtonAdded => 'Button added.';

  @override
  String get toastZipperAdded => 'Zipper added.';

  @override
  String get buttonClickPiece => 'Click on a piece.';

  @override
  String get trimsTitle => 'Buttons and zippers';

  @override
  String trimButton(String mm) {
    return 'Button · $mm mm';
  }

  @override
  String trimButtonhole(String mm, String piece) {
    return 'Buttonhole · $mm mm button on $piece';
  }

  @override
  String get trimButtoned => 'Buttoned';

  @override
  String trimZipper(String cm) {
    return 'Zipper · $cm cm';
  }

  @override
  String get trimOpen => 'Open';

  @override
  String get buttonAll => 'Button all';

  @override
  String get unbuttonAll => 'Unbutton all';

  @override
  String get trimRemove => 'Remove';

  @override
  String get tpTrim => 'Trim';

  @override
  String get tpTrimSize => 'Size';

  @override
  String get tpQuantity => 'Quantity';

  @override
  String get tpPlacement => 'On';

  @override
  String get tpButton => 'Button';

  @override
  String get tpZipper => 'Zipper';

  @override
  String get errOutsidePiece => 'Put it inside the piece.';

  @override
  String get bodyMenuMeasurements => 'Measurements…';

  @override
  String get bodyMenuImport => 'Import body…';

  @override
  String get bodyMenuCheck => 'Check body…';

  @override
  String get bodyMenuPoints => 'Show arrangement points';

  @override
  String get bodyMenuBuiltin => 'Use the built-in body';

  @override
  String get bodyFiles => '3D bodies (OBJ, glTF, GLB, PLY, STL)';

  @override
  String get bodyImported => 'Body imported: check it before draping.';

  @override
  String get bodyBuiltinBack => 'Back to the built-in body.';

  @override
  String get bodyCheckTitle => 'Check body';

  @override
  String bodyCheckSummary(String file, String triangles, String height) {
    return '$file · $triangles triangles · $height m tall';
  }

  @override
  String get bodyCheckReady => 'It can be draped.';

  @override
  String get bodyCheckNotReady => 'It cannot be draped yet: see below.';

  @override
  String get bodyCheckAllGood => 'Nothing to fix.';

  @override
  String get bodyReading => 'How the file is read';

  @override
  String get bodyUnits => 'Units';

  @override
  String get unitMetres => 'Metres';

  @override
  String get unitCentimetres => 'Centimetres';

  @override
  String get unitMillimetres => 'Millimetres';

  @override
  String get unitInches => 'Inches';

  @override
  String unitOther(String metres) {
    return '$metres m per unit';
  }

  @override
  String get bodyUp => 'Up axis';

  @override
  String get bodyTurn => 'Turn';

  @override
  String get bodyPoints => 'Arrangement points';

  @override
  String get bodyPointsHint =>
      'Inside the body, in cm: across (left +), height, depth (front +). Drag them in the 3D view with Body › Show arrangement points.';

  @override
  String get bodyGuessAgain => 'Guess again';

  @override
  String get bodyHasWarnings => 'The body has things to check.';

  @override
  String get pointChest => 'Chest';

  @override
  String get pointWaist => 'Waist';

  @override
  String get pointHips => 'Hips';

  @override
  String get pointCrotch => 'Crotch';

  @override
  String get pointLeftShoulder => 'Left shoulder';

  @override
  String get pointLeftElbow => 'Left elbow';

  @override
  String get pointLeftWrist => 'Left wrist';

  @override
  String get pointRightShoulder => 'Right shoulder';

  @override
  String get pointRightElbow => 'Right elbow';

  @override
  String get pointRightWrist => 'Right wrist';

  @override
  String get pointLeftThigh => 'Left thigh';

  @override
  String get pointLeftShin => 'Left shin';

  @override
  String get pointRightThigh => 'Right thigh';

  @override
  String get pointRightShin => 'Right shin';

  @override
  String checkFileUnavailable(String file) {
    return 'The body\'s file $file is missing or was changed: import it again.';
  }

  @override
  String checkMeshOpen(String holes, String edges) {
    return '$holes holes in the surface ($edges open edges): closed for the cloth as well as possible.';
  }

  @override
  String checkMeshNonManifold(String edges) {
    return '$edges edges are shared by more than two triangles.';
  }

  @override
  String checkMeshPieces(String pieces) {
    return '$pieces loose parts besides the body.';
  }

  @override
  String checkMeshHeavy(String triangles) {
    return '$triangles triangles: the views and the drape will be slower.';
  }

  @override
  String checkMeshCoarse(String triangles) {
    return 'Only $triangles triangles: the body will look faceted.';
  }

  @override
  String checkMeshReoriented(String triangles) {
    return '$triangles triangles were turned to face out like their neighbours.';
  }

  @override
  String get checkMeshInsideOut => 'The surface faced inward: it was turned.';

  @override
  String checkMeshDegenerate(String triangles) {
    return '$triangles flat triangles are ignored.';
  }

  @override
  String checkScaleHeight(String height) {
    return '$height m tall: are the units right?';
  }

  @override
  String get checkOrientationUp =>
      'The body is wider than it is tall: is the up axis right?';

  @override
  String get checkOrientationFacing =>
      'The feet point backwards: turn the body 180°.';

  @override
  String checkPointsOutside(String point) {
    return '$point is outside the body.';
  }

  @override
  String checkPointsSide(String point) {
    return '$point is on the wrong side of the body.';
  }

  @override
  String checkMeasurementsClamped(String measurement) {
    return 'A measurement ($measurement) was out of range and was brought into it.';
  }

  @override
  String get checkMotionFileSkeleton => 'Moves with the file\'s own skeleton.';

  @override
  String get checkMotionPoints =>
      'Moves with a skeleton placed by the arrangement points: move them if a limb bends in the wrong place.';

  @override
  String errModelMalformed(String format, String detail) {
    return 'This $format file cannot be read: $detail';
  }

  @override
  String get errModelExternalBuffers =>
      'This glTF keeps its data in separate files: save it as a single .glb.';

  @override
  String errModelCompressed(String extension) {
    return 'This glTF is compressed ($extension): save it without compression.';
  }

  @override
  String get errModelEmpty => 'This file holds no triangles.';

  @override
  String errModelTooLarge(String triangles, String limit) {
    return 'This body has $triangles triangles, more than $limit: simplify it first.';
  }

  @override
  String errBodyUnavailable(String file) {
    return 'The body\'s file $file is missing or was changed: import it again.';
  }

  @override
  String get errNoImportedBody => 'The avatar has no imported body.';

  @override
  String get exportGame => 'Garment for a game engine (glTF)…';

  @override
  String get gameTitle => 'Export for a game engine';

  @override
  String get gameIntro =>
      'The garment as the 3D view shows it, in one GLB file: lighter levels of detail, its wrinkles baked into normal maps, and moving with the body\'s skeleton.';

  @override
  String get gameEngine => 'Engine';

  @override
  String get gameGeneric => 'Generic';

  @override
  String get gameLods => 'Levels of detail';

  @override
  String get gameLodsHint =>
      'Percentages of the triangles, the first 100: for example 100, 50, 20. The edges of the pieces never move.';

  @override
  String get gameNormalMap => 'Normal map';

  @override
  String get gameNoMap => 'None';

  @override
  String get gameNormalMapHint =>
      'The full garment\'s wrinkles, baked for each lighter level.';

  @override
  String get gameSides => 'Back faces';

  @override
  String get gameDoubleSided => 'Double-sided';

  @override
  String get gameThickness => 'Thickness';

  @override
  String get gameThicknessMm => 'Thickness';

  @override
  String get gameSkin => 'Skin to the body\'s skeleton';

  @override
  String get gameSkinHint =>
      'The garment moves with the body, with the built-in motions and the project\'s captured motions as animations.';

  @override
  String get gameAvatar => 'Include the body';

  @override
  String get gameExport => 'Export…';

  @override
  String get mm => 'mm';

  @override
  String get errLods =>
      'Levels of detail: start at 100 and go down, each above 0, at most five.';

  @override
  String get errTextureSize =>
      'Choose no map, or a power of two from 256 to 4096.';

  @override
  String get errNoSkeleton =>
      'There is no avatar to skin the garment to: export without skinning.';

  @override
  String get motionImport => 'Import a captured motion…';

  @override
  String get motionFiles => 'Motion captures (BVH)';

  @override
  String errMotionFormat(String detail) {
    return 'This BVH file cannot be read: $detail';
  }

  @override
  String errMotionSkeleton(String bone) {
    return 'The skeleton of this motion has no joint for the $bone.';
  }

  @override
  String errMotionTooShort(String seconds) {
    return 'This motion is too short: it needs at least $seconds s.';
  }

  @override
  String get motionLibrary => 'Motion library';

  @override
  String get motionLibraryMenu => 'Motion library…';

  @override
  String get motionLibrarySubtitle =>
      'Captured motions (BVH files) you can use in every project.';

  @override
  String get motionLibraryEmpty =>
      'No motion yet: add BVH files here, or add a motion to the library when you import it.';

  @override
  String get motionLibraryAdd => 'Add files…';

  @override
  String get motionLibraryPlay => 'Play on the avatar';

  @override
  String motionLibraryDeleteConfirm(String name) {
    return 'Delete “$name” from the library? Projects that use it keep their copy.';
  }

  @override
  String get motionImported => 'Motion imported into the project.';

  @override
  String get motionAddToLibrary => 'Add to library';

  @override
  String motionAddedToLibrary(String name) {
    return 'Added to the library: $name';
  }

  @override
  String errLibraryName(String name) {
    return '“$name” cannot name a motion: avoid / \\ : * ? \" < > | and a leading or trailing dot.';
  }

  @override
  String errLibraryExists(String name) {
    return 'The library already has a motion named “$name”.';
  }

  @override
  String errLibraryMissing(String name) {
    return 'The library has no motion named “$name” any more.';
  }

  @override
  String errLibraryIo(String detail) {
    return 'The motion library cannot be read or written: $detail';
  }

  @override
  String get barElastic => 'Elastic';

  @override
  String get barElasticHint =>
      'Sew an elastic along this edge: it gathers when draped.';

  @override
  String get barShirring => 'Shirring';

  @override
  String get barShirringHint =>
      'Rows of elastic across the piece, parallel to this edge.';

  @override
  String get elasticTitle => 'Elastic';

  @override
  String get elasticRatio => 'Relaxed length';

  @override
  String get elasticWidth => 'Width';

  @override
  String get shirringTitle => 'Shirring';

  @override
  String get shirringRows => 'Rows';

  @override
  String get shirringFirst => 'First row from the edge';

  @override
  String get shirringGap => 'Between rows';

  @override
  String get detailsTitle => 'Construction details';

  @override
  String detailElastic(String cm, String width) {
    return 'Elastic · $cm cm, $width mm wide';
  }

  @override
  String get tpElastic => 'Elastic';

  @override
  String get tpElasticBySize => 'Elastic per garment by size (cm)';

  @override
  String get errHasElastic => 'This stretch already has an elastic.';

  @override
  String get barTopstitch => 'Topstitch';

  @override
  String get barTopstitchHint =>
      'Rows of stitches along this edge, seen on the right side.';

  @override
  String get topstitchTitle => 'Topstitch';

  @override
  String get topstitchRows => 'Rows';

  @override
  String get topstitchDistance => 'From the edge';

  @override
  String get topstitchGap => 'Between rows';

  @override
  String get topstitchStitch => 'Stitch length';

  @override
  String get threadNormal => 'Sewing thread';

  @override
  String get threadHeavy => 'Topstitching thread';

  @override
  String detailTopstitch(int rows, String mm, String stitch) {
    String _temp0 = intl.Intl.pluralLogic(
      rows,
      locale: localeName,
      other: '$rows rows',
      one: '1 row',
    );
    return '$_temp0 · $mm mm from the edge, $stitch mm stitches';
  }

  @override
  String get errOtherKind => 'That is another kind of detail.';

  @override
  String get tpTopstitching => 'Topstitching';

  @override
  String get tpRows => 'Rows';

  @override
  String get tpRowDistance => 'From the edge (mm)';

  @override
  String get tpRowGap => 'Between rows (mm)';

  @override
  String get tpStitch => 'Stitch (mm)';

  @override
  String get tpSpi => 'Stitches per inch';

  @override
  String get tpThread => 'Thread';

  @override
  String get tpThreadNormal => 'normal';

  @override
  String get tpThreadHeavy => 'heavy';

  @override
  String get tpElastics => 'Elastics';

  @override
  String get tpRatio => 'Relaxed length (%)';

  @override
  String get tpElasticWidth => 'Width (mm)';

  @override
  String get tpRelaxed => 'Relaxed (cm)';

  @override
  String get barInterfacing => 'Interfacing';

  @override
  String get barInterfacingHint =>
      'Fuse an interfacing on the whole piece: stiffer, it holds its shape.';

  @override
  String get barInterfacingBandHint =>
      'Fuse an interfacing in a band along this edge (a facing, a waistband).';

  @override
  String get interfacingTitle => 'Interfacing';

  @override
  String get interfacingWidth => 'Band width';

  @override
  String get interfacingLight => 'light';

  @override
  String get interfacingMedium => 'medium';

  @override
  String get interfacingHeavy => 'heavy';

  @override
  String detailInterfacing(String weight, String cm2) {
    return 'Interfacing, $weight · $cm2 cm²';
  }

  @override
  String get errNotClosed => 'Use a closed line on this piece.';

  @override
  String get tpInterfacing => 'Interfacing';

  @override
  String get tpInterfacingFabric => 'Kind';

  @override
  String get tpInterfacingWeight => 'Weight';

  @override
  String get tpInterfacingArea => 'Area (m²)';

  @override
  String get tpWoven => 'woven';

  @override
  String get tpNonWoven => 'non-woven';

  @override
  String get tpKnit => 'knit';

  @override
  String get barQuilting => 'Quilting';

  @override
  String get barQuiltingHint =>
      'Quilt the piece over a padding: it puffs between the stitched lines.';

  @override
  String get quiltingTitle => 'Quilting';

  @override
  String get quiltingSpacing => 'Between lines';

  @override
  String get quiltingLoft => 'Padding';

  @override
  String get quiltingDiamonds => 'diamonds';

  @override
  String get quiltingChannels => 'channels';

  @override
  String detailQuilting(String pattern, String cm, String mm) {
    return 'Quilted in $pattern · $cm cm apart, $mm mm padding';
  }

  @override
  String get tpPadding => 'Padding';

  @override
  String get tpLoft => 'Loft (mm)';

  @override
  String get tpPaddingWeight => 'Weight (g/m²)';

  @override
  String get tpQuilting => 'Quilting';

  @override
  String get tpPatternKind => 'Pattern';

  @override
  String get tpAngle => 'Angle (°)';

  @override
  String get tpSpacing => 'Spacing (mm)';

  @override
  String get templateJeans => 'Jeans';

  @override
  String get templateJeansBlurb =>
      'Denim, five pockets, a back yoke, a fly and belt loops.';

  @override
  String get templateShorts => 'Shorts';

  @override
  String get templateShortsBlurb =>
      'The trousers\' block, ending a hand above the knee.';

  @override
  String get libraryOpen => 'From the library…';

  @override
  String get libraryManage => 'Library';

  @override
  String get libraryManageSubtitle =>
      'Your own patterns: rename, export or delete them, or import one. Save a garment here from its card in a project, and add a pattern to a project from there.';

  @override
  String get libraryTitle => 'Pattern library';

  @override
  String get librarySubtitle =>
      'Click a pattern to add it to this project, drafted for its body. Save your own garments here from their card.';

  @override
  String get libraryKindTops => 'Tops';

  @override
  String get libraryKindBottoms => 'Bottoms';

  @override
  String get libraryKindDresses => 'Dresses and skirts';

  @override
  String get libraryKindAccessories => 'Accessories';

  @override
  String get libraryMine => 'My patterns';

  @override
  String get libraryMineEmpty =>
      'None yet: save a garment from its card (the bookmark button) to add it to other projects.';

  @override
  String get libraryImport => 'Import…';

  @override
  String get libraryExport => 'Export…';

  @override
  String get libraryPatternFiles => 'Pattern files';

  @override
  String libraryDeleteConfirm(String name) {
    return 'Delete “$name” from the library? Projects it was added to keep their garments.';
  }

  @override
  String libraryReplaceConfirm(String name) {
    return 'The library already has a pattern called “$name”. Replace it?';
  }

  @override
  String get libraryReplace => 'Replace';

  @override
  String get librarySave => 'Save to library';

  @override
  String get librarySaveTitle => 'Save to the pattern library';

  @override
  String get librarySaveName => 'Name';

  @override
  String get librarySaveKind => 'Kind';

  @override
  String librarySaved(String name) {
    return 'Saved to the library as “$name”.';
  }

  @override
  String get busyLibraryAdd => 'Adding the pattern';

  @override
  String get errLibraryNone => 'No pattern library is set up.';

  @override
  String errPatternExists(String name) {
    return 'The library already has a pattern called “$name”.';
  }

  @override
  String errPatternMissing(String name) {
    return 'The library has no pattern called “$name” any more.';
  }

  @override
  String get templateSweatshirt => 'Sweatshirt';

  @override
  String get templateSweatshirtBlurb =>
      'The hoodie without its hood: a crew neck, gathered at the wrists and the hem.';

  @override
  String get templateBodysuit => 'Baby bodysuit';

  @override
  String get templateBodysuitBlurb =>
      'Jersey, short sleeves, closed under the crotch, with leg openings.';

  @override
  String get templatePyjama => 'Pyjama';

  @override
  String get templatePyjamaBlurb =>
      'Top and bottoms in jersey: two garments, the top worn over the bottoms.';

  @override
  String get templatePyjamaTop => 'Pyjama top';

  @override
  String get templatePyjamaTopBlurb =>
      'Loose, long sleeves gathered at the wrists.';

  @override
  String get templatePyjamaBottoms => 'Pyjama bottoms';

  @override
  String get templatePyjamaBottomsBlurb =>
      'Loose, pulled on: an elastic waist, open at the ankles.';

  @override
  String get templateJoggingSuit => 'Jogging suit';

  @override
  String get templateJoggingSuitBlurb =>
      'Joggers and a sweatshirt worn over them: two garments.';

  @override
  String get bodyAdult => 'Adult';

  @override
  String get bodyChild => 'Child';

  @override
  String get bodyAge => 'Age';

  @override
  String get bodyAgeYears => 'years';

  @override
  String get bodyGirl => 'Girl';

  @override
  String get bodyBoy => 'Boy';

  @override
  String get chartGirls => 'Girls';

  @override
  String get chartBoys => 'Boys';

  @override
  String get chartBabies => 'Babies';

  @override
  String get libraryKindChildren => 'Babies and children';

  @override
  String presetGirls(String size) {
    return 'Girls $size y';
  }

  @override
  String presetBoys(String size) {
    return 'Boys $size y';
  }

  @override
  String presetBaby(String months) {
    return 'Baby $months months';
  }

  @override
  String get newProjectTitle => 'New project';

  @override
  String get kindWorn => 'Garment';

  @override
  String get kindWornBlurb => 'Worn: draped on a body';

  @override
  String get kindObject => 'Object';

  @override
  String get kindObjectBlurb => 'No body: a bag, a cushion, an accessory';

  @override
  String get garmentSizes => 'Garment measurements';

  @override
  String get templateCushion => 'Cushion';

  @override
  String get templateCushionBlurb => 'Two squares sewn all around, stuffed.';

  @override
  String sizeYears(String age) {
    return '$age years';
  }

  @override
  String sizeMonths(String months) {
    return '$months months';
  }

  @override
  String get bodySize => 'Size';

  @override
  String get silhouette => 'Silhouette';

  @override
  String get silhouetteBalanced => 'Standard';

  @override
  String get silhouetteBalancedHint => 'The size chart\'s proportions';

  @override
  String get silhouetteHourglass => 'Hourglass';

  @override
  String get silhouetteHourglassHint => 'A marked waist';

  @override
  String get silhouettePear => 'Pear';

  @override
  String get silhouettePearHint => 'Hips fuller than the bust';

  @override
  String get silhouetteStraight => 'Straight';

  @override
  String get silhouetteStraightHint => 'Little waist, narrower hips';

  @override
  String get silhouetteRound => 'Round';

  @override
  String get silhouetteRoundHint => 'Fuller at the waist';

  @override
  String get silhouetteInverted => 'Inverted triangle';

  @override
  String get silhouetteInvertedHint => 'Narrower hips under a broad chest';

  @override
  String get silhouetteSlim => 'Slim';

  @override
  String get silhouetteSlimHint => 'Slimmer than the chart for this age';

  @override
  String get silhouetteSturdy => 'Sturdy';

  @override
  String get silhouetteSturdyHint => 'Fuller than the chart for this age';

  @override
  String get myBodies => 'My bodies';

  @override
  String get myBodiesTip => 'Start from one of your saved bodies';

  @override
  String chartMeasurements(String chart) {
    return 'The $chart size chart\'s measurements: change them if you like.';
  }

  @override
  String get typedMeasurements => 'Your measurements.';

  @override
  String typedNearestSize(String size) {
    return 'Your measurements make a size $size.';
  }

  @override
  String nearestSizeOn(String size, String chart) {
    return 'Nearest size: $size ($chart)';
  }

  @override
  String get stuffing => 'Stuffing';

  @override
  String get stuffingNone => 'None';

  @override
  String get stuffingFirm => 'Firm';

  @override
  String get stuffingHint =>
      'Only parts closed all around are stuffed (a cushion, a toy); an open bag keeps the shape of its seams.';

  @override
  String get objectTitle => 'Object';

  @override
  String get objectSubtitle =>
      'No body: in 3D the object floats without weight, in the shape its seams give it.';

  @override
  String get objectWear => 'Wear it on a body';

  @override
  String get objectMade =>
      'The project is now an object: no body, no weight in 3D.';

  @override
  String get bodyMenuObject => 'Make it an object (no body)';

  @override
  String get errObject =>
      'This project is an object, without a body: wear it on a body first.';

  @override
  String get inch => 'in';

  @override
  String get sizeSystemEu => 'EU';

  @override
  String get sizeSystemUs => 'US';

  @override
  String get sizeSystemUk => 'UK';

  @override
  String get sizeSystemLetters => 'S·M·L';

  @override
  String get sizeSystemTip =>
      'How sizes are written: European (38), American (6), British (10) or letters (M). Remembered for next time.';

  @override
  String get unitsTip =>
      'Lengths in centimetres or inches. Remembered for next time.';
}
