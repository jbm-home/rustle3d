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
  String appVersion(String version) {
    return 'Version $version';
  }

  @override
  String get updatesLink => 'Updates and news on rustle3d.com';

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
  String get toolSewHint => 'Click two edges on the pattern';

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
      'Corner picked. ⇧-click another corner to measure the distance between them.';

  @override
  String get pointsPicked =>
      'Between the two corners. The second one moves along the line.';

  @override
  String get straighten => 'Straighten';

  @override
  String get hintSewFirst =>
      'Sew: click the first edge on the pattern · V to stop';

  @override
  String get hintSewSecond =>
      'Sew: now click the edge to sew it to · V to stop';

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
  String statsSim(int points, String stepMs, String time) {
    return 'Cloth $points points · step $stepMs ms · $time s · ';
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
}
