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
  String get updatesLink => 'Updates and news on rustle3d.com';

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
  String measBetween(String measure, int min, int max) {
    return '$measure must be between $min and $max cm.';
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
  String get allProjects => 'All projects';

  @override
  String get export => 'Export';

  @override
  String get importPattern => 'Import';

  @override
  String get importTip =>
      'Import a pattern drawn elsewhere: DXF-AAMA, DXF, SVG or PDF';

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
  String get agentsHow => 'An agent connects with: r3d mcp --attach';

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
  String get tourGuideTitle => 'One step at a time';

  @override
  String get tourGuideText =>
      'The guide says what to do next. Click a step at the top to jump to it, or close the guide.';

  @override
  String get tourHelpTitle => 'Help is always here';

  @override
  String get tourHelpText =>
      '**Learn** has short how-tos and shows where things are; **Body** changes the measurements. **⌘Z** undoes anything.';

  @override
  String get stepMeasurements => 'Measurements';

  @override
  String get stepMeasurementsGoal =>
      'Check the body the garment is made for: its size and measurements.';

  @override
  String get stepMeasurementsAction => 'Open Body';

  @override
  String get stepPattern => 'Pattern';

  @override
  String get stepPatternGoal =>
      'Shape the pieces: drag a corner or an edge, or click an edge to type its length.';

  @override
  String get stepPatternAction => 'Add a piece';

  @override
  String get stepSew => 'Sew';

  @override
  String get stepSewGoal =>
      'Join the pieces: click an edge on the pattern, then the edge to sew it to.';

  @override
  String get stepSewAction => 'Sew tool';

  @override
  String get stepFabric => 'Fabric';

  @override
  String get stepFabricGoal =>
      'Choose what the garment is made of, in the panel on the right.';

  @override
  String get stepDrape => 'Drape';

  @override
  String get stepDrapeGoal =>
      'Watch it drape on the body. Then drag the cloth, or ⌥-click to pin it.';

  @override
  String stepOf(int step, int total) {
    return 'Step $step of $total';
  }

  @override
  String get guideNext => 'Next  →';

  @override
  String get guideHide => 'Hide the guide (click a step to bring it back)';

  @override
  String get toolSelect => 'Select';

  @override
  String get toolSelectHint =>
      'Click a piece; drag a corner or an edge to reshape it';

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
  String get toolDraw => 'Draw on garment';

  @override
  String get toolDrawHint =>
      'Drag on the 3D garment: the line goes on the pattern';

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
  String get errSewn => 'This edge is sewn: unpick the seam first.';

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
  String get photoTip => 'Render a photorealistic image of the garment';

  @override
  String get photoTitle => 'Render a photo';

  @override
  String get photoIntro =>
      'A photorealistic image of the garment as the 3D view shows it now, in a photo studio.';

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
  String get photoCloseTip => 'Back to the live 3D view';

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
      'project_rename': 'Rename project',
      'project_set_display_unit': 'Change units',
      'avatar_set': 'Change body',
      'avatar_remove': 'Remove body',
      'panel_create_rect': 'Add rectangle',
      'panel_create_polygon': 'Add polygon',
      'panel_create_path': 'Add curved piece',
      'panel_rename': 'Rename piece',
      'panel_translate': 'Move piece',
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
      'other': '$kind',
    });
    return '$_temp0';
  }

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
  String get toolDetails => 'Details';

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
  String get combine => 'Join or cut';

  @override
  String get combineWith => 'With the piece';

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
  String get errNotJoined =>
      'These edges do not follow each other on one piece.';
}
