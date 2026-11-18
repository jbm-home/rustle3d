import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_fr.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of L
/// returned by `L.of(context)`.
///
/// Applications need to include `L.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: L.localizationsDelegates,
///   supportedLocales: L.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the L.supportedLocales
/// property.
abstract class L {
  L(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static L of(BuildContext context) {
    return Localizations.of<L>(context, L)!;
  }

  static const LocalizationsDelegate<L> delegate = _LDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('fr'),
  ];

  /// No description provided for @languageTooltip.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get languageTooltip;

  /// No description provided for @themeLight.
  ///
  /// In en, this message translates to:
  /// **'Light theme'**
  String get themeLight;

  /// No description provided for @themeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark theme'**
  String get themeDark;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @create.
  ///
  /// In en, this message translates to:
  /// **'Create'**
  String get create;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @apply.
  ///
  /// In en, this message translates to:
  /// **'Apply'**
  String get apply;

  /// No description provided for @delete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get delete;

  /// No description provided for @next.
  ///
  /// In en, this message translates to:
  /// **'Next'**
  String get next;

  /// No description provided for @skip.
  ///
  /// In en, this message translates to:
  /// **'Skip'**
  String get skip;

  /// No description provided for @start.
  ///
  /// In en, this message translates to:
  /// **'Start'**
  String get start;

  /// No description provided for @gotIt.
  ///
  /// In en, this message translates to:
  /// **'Got it'**
  String get gotIt;

  /// No description provided for @cm.
  ///
  /// In en, this message translates to:
  /// **'cm'**
  String get cm;

  /// No description provided for @keyCtrl.
  ///
  /// In en, this message translates to:
  /// **'Ctrl'**
  String get keyCtrl;

  /// No description provided for @keyAlt.
  ///
  /// In en, this message translates to:
  /// **'Alt'**
  String get keyAlt;

  /// No description provided for @keyShift.
  ///
  /// In en, this message translates to:
  /// **'Shift'**
  String get keyShift;

  /// No description provided for @keyDelete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get keyDelete;

  /// No description provided for @startTitle.
  ///
  /// In en, this message translates to:
  /// **'What will you make?'**
  String get startTitle;

  /// No description provided for @templateTShirt.
  ///
  /// In en, this message translates to:
  /// **'T-shirt'**
  String get templateTShirt;

  /// No description provided for @templateTShirtBlurb.
  ///
  /// In en, this message translates to:
  /// **'Front, back and short sleeves, in jersey.'**
  String get templateTShirtBlurb;

  /// No description provided for @templateShirt.
  ///
  /// In en, this message translates to:
  /// **'Shirt'**
  String get templateShirt;

  /// No description provided for @templateShirtBlurb.
  ///
  /// In en, this message translates to:
  /// **'Long sleeves with cuffs and a stand collar.'**
  String get templateShirtBlurb;

  /// No description provided for @templateTrousers.
  ///
  /// In en, this message translates to:
  /// **'Trousers'**
  String get templateTrousers;

  /// No description provided for @templateTrousersBlurb.
  ///
  /// In en, this message translates to:
  /// **'Straight legs from the waist to the ankle.'**
  String get templateTrousersBlurb;

  /// No description provided for @templateDress.
  ///
  /// In en, this message translates to:
  /// **'Dress'**
  String get templateDress;

  /// No description provided for @templateDressBlurb.
  ///
  /// In en, this message translates to:
  /// **'Sleeveless, gently shaped, knee length.'**
  String get templateDressBlurb;

  /// No description provided for @templateStraightSkirt.
  ///
  /// In en, this message translates to:
  /// **'Straight skirt'**
  String get templateStraightSkirt;

  /// No description provided for @templateStraightSkirtBlurb.
  ///
  /// In en, this message translates to:
  /// **'Close over the hips, from the waist to the knee.'**
  String get templateStraightSkirtBlurb;

  /// No description provided for @templateALineSkirt.
  ///
  /// In en, this message translates to:
  /// **'A-line skirt'**
  String get templateALineSkirt;

  /// No description provided for @templateALineSkirtBlurb.
  ///
  /// In en, this message translates to:
  /// **'Flares gently from the hips to the hem.'**
  String get templateALineSkirtBlurb;

  /// No description provided for @templateToteBag.
  ///
  /// In en, this message translates to:
  /// **'Tote bag'**
  String get templateToteBag;

  /// No description provided for @templateToteBagBlurb.
  ///
  /// In en, this message translates to:
  /// **'Two pieces and four seams: the easiest start.'**
  String get templateToteBagBlurb;

  /// No description provided for @sectionNew.
  ///
  /// In en, this message translates to:
  /// **'New'**
  String get sectionNew;

  /// No description provided for @openFromDisk.
  ///
  /// In en, this message translates to:
  /// **'Open…'**
  String get openFromDisk;

  /// No description provided for @open.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get open;

  /// No description provided for @allProjectsCount.
  ///
  /// In en, this message translates to:
  /// **'All projects ({count})'**
  String allProjectsCount(int count);

  /// No description provided for @projectsFolder.
  ///
  /// In en, this message translates to:
  /// **'Projects folder'**
  String get projectsFolder;

  /// No description provided for @importProjectZip.
  ///
  /// In en, this message translates to:
  /// **'Import .zip…'**
  String get importProjectZip;

  /// No description provided for @importProjectZipTip.
  ///
  /// In en, this message translates to:
  /// **'Add a project someone sent you as a .zip archive'**
  String get importProjectZipTip;

  /// No description provided for @projectZipFiles.
  ///
  /// In en, this message translates to:
  /// **'Project archives'**
  String get projectZipFiles;

  /// No description provided for @projectOptions.
  ///
  /// In en, this message translates to:
  /// **'Project options'**
  String get projectOptions;

  /// No description provided for @renameProject.
  ///
  /// In en, this message translates to:
  /// **'Rename…'**
  String get renameProject;

  /// No description provided for @exportProjectZip.
  ///
  /// In en, this message translates to:
  /// **'Export as .zip…'**
  String get exportProjectZip;

  /// No description provided for @deleteProject.
  ///
  /// In en, this message translates to:
  /// **'Delete…'**
  String get deleteProject;

  /// No description provided for @renameProjectTitle.
  ///
  /// In en, this message translates to:
  /// **'Rename the project'**
  String get renameProjectTitle;

  /// No description provided for @rename.
  ///
  /// In en, this message translates to:
  /// **'Rename'**
  String get rename;

  /// No description provided for @deleteProjectTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete “{name}”?'**
  String deleteProjectTitle(String name);

  /// No description provided for @deleteProjectBody.
  ///
  /// In en, this message translates to:
  /// **'The project folder and everything in it (pattern, body, exports) will be deleted from this computer. This cannot be undone.'**
  String get deleteProjectBody;

  /// No description provided for @deleteProjectConfirm.
  ///
  /// In en, this message translates to:
  /// **'I understand that this project will be deleted for good'**
  String get deleteProjectConfirm;

  /// No description provided for @projectDeleted.
  ///
  /// In en, this message translates to:
  /// **'Deleted “{name}”.'**
  String projectDeleted(String name);

  /// No description provided for @projectExported.
  ///
  /// In en, this message translates to:
  /// **'Exported “{name}” to {file}.'**
  String projectExported(String name, String file);

  /// No description provided for @appVersion.
  ///
  /// In en, this message translates to:
  /// **'Version {version}'**
  String appVersion(String version);

  /// No description provided for @updatesLink.
  ///
  /// In en, this message translates to:
  /// **'Updates and news on rustle3d.com'**
  String get updatesLink;

  /// No description provided for @cannotOpen.
  ///
  /// In en, this message translates to:
  /// **'Could not open {target}.'**
  String cannotOpen(String target);

  /// No description provided for @blankProject.
  ///
  /// In en, this message translates to:
  /// **'Blank project'**
  String get blankProject;

  /// No description provided for @blankProjectBlurb.
  ///
  /// In en, this message translates to:
  /// **'Draw your own pieces.'**
  String get blankProjectBlurb;

  /// No description provided for @untitled.
  ///
  /// In en, this message translates to:
  /// **'Untitled'**
  String get untitled;

  /// No description provided for @recent.
  ///
  /// In en, this message translates to:
  /// **'Recent'**
  String get recent;

  /// No description provided for @recentEmpty.
  ///
  /// In en, this message translates to:
  /// **'Your projects will appear here. They are saved as you work, in Documents › Rustle3D.'**
  String get recentEmpty;

  /// No description provided for @agoJustNow.
  ///
  /// In en, this message translates to:
  /// **'just now'**
  String get agoJustNow;

  /// No description provided for @agoMinutes.
  ///
  /// In en, this message translates to:
  /// **'{count} min ago'**
  String agoMinutes(int count);

  /// No description provided for @agoHours.
  ///
  /// In en, this message translates to:
  /// **'{count} h ago'**
  String agoHours(int count);

  /// No description provided for @agoYesterday.
  ///
  /// In en, this message translates to:
  /// **'yesterday'**
  String get agoYesterday;

  /// No description provided for @agoDays.
  ///
  /// In en, this message translates to:
  /// **'{count} days ago'**
  String agoDays(int count);

  /// No description provided for @newGarmentTitle.
  ///
  /// In en, this message translates to:
  /// **'New: {garment}'**
  String newGarmentTitle(String garment);

  /// No description provided for @fieldName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get fieldName;

  /// No description provided for @fieldBody.
  ///
  /// In en, this message translates to:
  /// **'Body'**
  String get fieldBody;

  /// No description provided for @sizeMustBePositive.
  ///
  /// In en, this message translates to:
  /// **'{size} must be a positive number of cm.'**
  String sizeMustBePositive(String size);

  /// No description provided for @sizeChest.
  ///
  /// In en, this message translates to:
  /// **'Chest'**
  String get sizeChest;

  /// No description provided for @sizeChestHint.
  ///
  /// In en, this message translates to:
  /// **'Around the fullest part of the chest'**
  String get sizeChestHint;

  /// No description provided for @sizeWaist.
  ///
  /// In en, this message translates to:
  /// **'Waist'**
  String get sizeWaist;

  /// No description provided for @sizeWaistHint.
  ///
  /// In en, this message translates to:
  /// **'Around the narrowest part'**
  String get sizeWaistHint;

  /// No description provided for @sizeHips.
  ///
  /// In en, this message translates to:
  /// **'Hips'**
  String get sizeHips;

  /// No description provided for @sizeHipsHint.
  ///
  /// In en, this message translates to:
  /// **'Around the fullest part'**
  String get sizeHipsHint;

  /// No description provided for @sizeLength.
  ///
  /// In en, this message translates to:
  /// **'Length'**
  String get sizeLength;

  /// No description provided for @sizeLengthHint.
  ///
  /// In en, this message translates to:
  /// **'Neck to hem for tops, waist to hem for skirts, height of a bag'**
  String get sizeLengthHint;

  /// No description provided for @sizeSleeve.
  ///
  /// In en, this message translates to:
  /// **'Sleeve'**
  String get sizeSleeve;

  /// No description provided for @sizeSleeveHint.
  ///
  /// In en, this message translates to:
  /// **'From the top of the shoulder'**
  String get sizeSleeveHint;

  /// No description provided for @sizeWidth.
  ///
  /// In en, this message translates to:
  /// **'Width'**
  String get sizeWidth;

  /// No description provided for @sizeWidthHint.
  ///
  /// In en, this message translates to:
  /// **'Across the bag'**
  String get sizeWidthHint;

  /// No description provided for @sizeEase.
  ///
  /// In en, this message translates to:
  /// **'Ease'**
  String get sizeEase;

  /// No description provided for @sizeEaseHint.
  ///
  /// In en, this message translates to:
  /// **'Room added to the body'**
  String get sizeEaseHint;

  /// No description provided for @pieceFront.
  ///
  /// In en, this message translates to:
  /// **'Front'**
  String get pieceFront;

  /// No description provided for @pieceBack.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get pieceBack;

  /// No description provided for @pieceBackLeft.
  ///
  /// In en, this message translates to:
  /// **'Back left'**
  String get pieceBackLeft;

  /// No description provided for @pieceBackRight.
  ///
  /// In en, this message translates to:
  /// **'Back right'**
  String get pieceBackRight;

  /// No description provided for @pieceSleeveLeft.
  ///
  /// In en, this message translates to:
  /// **'Sleeve left'**
  String get pieceSleeveLeft;

  /// No description provided for @pieceSleeveRight.
  ///
  /// In en, this message translates to:
  /// **'Sleeve right'**
  String get pieceSleeveRight;

  /// No description provided for @pieceNew.
  ///
  /// In en, this message translates to:
  /// **'Panel {number}'**
  String pieceNew(int number);

  /// No description provided for @pieceMirror.
  ///
  /// In en, this message translates to:
  /// **'{name} (mirror)'**
  String pieceMirror(String name);

  /// No description provided for @body.
  ///
  /// In en, this message translates to:
  /// **'Body'**
  String get body;

  /// No description provided for @bodySubtitle.
  ///
  /// In en, this message translates to:
  /// **'The 3D body is fitted to these measurements. Start from a size, then adjust.'**
  String get bodySubtitle;

  /// No description provided for @bodyPreset.
  ///
  /// In en, this message translates to:
  /// **'Preset'**
  String get bodyPreset;

  /// No description provided for @presetWomen.
  ///
  /// In en, this message translates to:
  /// **'Women {size}'**
  String presetWomen(String size);

  /// No description provided for @presetMen.
  ///
  /// In en, this message translates to:
  /// **'Men {size}'**
  String presetMen(String size);

  /// No description provided for @bodyFeminine.
  ///
  /// In en, this message translates to:
  /// **'Feminine body'**
  String get bodyFeminine;

  /// No description provided for @bodyMasculine.
  ///
  /// In en, this message translates to:
  /// **'Masculine body'**
  String get bodyMasculine;

  /// No description provided for @measHeight.
  ///
  /// In en, this message translates to:
  /// **'Height'**
  String get measHeight;

  /// No description provided for @measBust.
  ///
  /// In en, this message translates to:
  /// **'Bust / chest'**
  String get measBust;

  /// No description provided for @measBetween.
  ///
  /// In en, this message translates to:
  /// **'{measure} must be between {min} and {max} cm.'**
  String measBetween(String measure, int min, int max);

  /// No description provided for @presetName.
  ///
  /// In en, this message translates to:
  /// **'Preset name'**
  String get presetName;

  /// No description provided for @saveAsPreset.
  ///
  /// In en, this message translates to:
  /// **'Save as preset'**
  String get saveAsPreset;

  /// No description provided for @deletePreset.
  ///
  /// In en, this message translates to:
  /// **'Delete preset'**
  String get deletePreset;

  /// No description provided for @fittedBody.
  ///
  /// In en, this message translates to:
  /// **'Fitted body: height {height} · bust {bust} · waist {waist} · hips {hips}'**
  String fittedBody(String height, String bust, String waist, String hips);

  /// No description provided for @allProjects.
  ///
  /// In en, this message translates to:
  /// **'All projects'**
  String get allProjects;

  /// No description provided for @export.
  ///
  /// In en, this message translates to:
  /// **'Export'**
  String get export;

  /// No description provided for @importPattern.
  ///
  /// In en, this message translates to:
  /// **'Import'**
  String get importPattern;

  /// No description provided for @importTip.
  ///
  /// In en, this message translates to:
  /// **'Import a pattern drawn elsewhere: DXF-AAMA, DXF, SVG or PDF'**
  String get importTip;

  /// No description provided for @importFiles.
  ///
  /// In en, this message translates to:
  /// **'Patterns'**
  String get importFiles;

  /// No description provided for @imported.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 piece imported} other{{count} pieces imported}}.'**
  String imported(int count);

  /// No description provided for @importIgnored.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 shape was not part of a piece.} other{{count} shapes were not part of a piece.}}'**
  String importIgnored(int count);

  /// No description provided for @importUnitsTitle.
  ///
  /// In en, this message translates to:
  /// **'What unit was this drawn in?'**
  String get importUnitsTitle;

  /// No description provided for @importUnitsBody.
  ///
  /// In en, this message translates to:
  /// **'The file does not give its scale, so the pieces may be the wrong size. Choose the unit it was drawn in, or keep them as they are.'**
  String get importUnitsBody;

  /// No description provided for @importKeep.
  ///
  /// In en, this message translates to:
  /// **'Keep as is'**
  String get importKeep;

  /// No description provided for @unitMm.
  ///
  /// In en, this message translates to:
  /// **'Millimetres'**
  String get unitMm;

  /// No description provided for @unitCm.
  ///
  /// In en, this message translates to:
  /// **'Centimetres'**
  String get unitCm;

  /// No description provided for @unitIn.
  ///
  /// In en, this message translates to:
  /// **'Inches'**
  String get unitIn;

  /// No description provided for @unitPt.
  ///
  /// In en, this message translates to:
  /// **'Points (1/72 inch)'**
  String get unitPt;

  /// No description provided for @unitPx.
  ///
  /// In en, this message translates to:
  /// **'Pixels (1/96 inch)'**
  String get unitPx;

  /// No description provided for @agents.
  ///
  /// In en, this message translates to:
  /// **'Agents'**
  String get agents;

  /// No description provided for @agentsWaiting.
  ///
  /// In en, this message translates to:
  /// **'Agents: waiting'**
  String get agentsWaiting;

  /// No description provided for @agentConnected.
  ///
  /// In en, this message translates to:
  /// **'Agent connected'**
  String get agentConnected;

  /// No description provided for @agentsTip.
  ///
  /// In en, this message translates to:
  /// **'AI agents: let one edit this project live'**
  String get agentsTip;

  /// No description provided for @allowAgents.
  ///
  /// In en, this message translates to:
  /// **'Allow AI agents'**
  String get allowAgents;

  /// No description provided for @agentsHow.
  ///
  /// In en, this message translates to:
  /// **'An agent connects with: r3d mcp --attach'**
  String get agentsHow;

  /// No description provided for @disconnectAgent.
  ///
  /// In en, this message translates to:
  /// **'Disconnect the agent'**
  String get disconnectAgent;

  /// No description provided for @exportSvg.
  ///
  /// In en, this message translates to:
  /// **'Pattern, SVG at 1:1'**
  String get exportSvg;

  /// No description provided for @exportPdfA4.
  ///
  /// In en, this message translates to:
  /// **'Pattern, PDF on A4 pages'**
  String get exportPdfA4;

  /// No description provided for @exportPdfSingle.
  ///
  /// In en, this message translates to:
  /// **'Pattern, PDF on one page'**
  String get exportPdfSingle;

  /// No description provided for @exportGlb.
  ///
  /// In en, this message translates to:
  /// **'Garment in 3D, glTF (.glb)'**
  String get exportGlb;

  /// No description provided for @exportGlbAvatar.
  ///
  /// In en, this message translates to:
  /// **'Garment and body in 3D, glTF (.glb)'**
  String get exportGlbAvatar;

  /// No description provided for @exportObj.
  ///
  /// In en, this message translates to:
  /// **'Garment in 3D, OBJ'**
  String get exportObj;

  /// No description provided for @exportDxf.
  ///
  /// In en, this message translates to:
  /// **'Pattern, DXF-AAMA (pattern software)'**
  String get exportDxf;

  /// No description provided for @exported.
  ///
  /// In en, this message translates to:
  /// **'Exported {file}.'**
  String exported(String file);

  /// No description provided for @learn.
  ///
  /// In en, this message translates to:
  /// **'Learn'**
  String get learn;

  /// No description provided for @undoTip.
  ///
  /// In en, this message translates to:
  /// **'Undo  ⌘Z'**
  String get undoTip;

  /// No description provided for @redoTip.
  ///
  /// In en, this message translates to:
  /// **'Redo  ⇧⌘Z'**
  String get redoTip;

  /// No description provided for @tourPatternTitle.
  ///
  /// In en, this message translates to:
  /// **'Your pattern'**
  String get tourPatternTitle;

  /// No description provided for @tourPatternText.
  ///
  /// In en, this message translates to:
  /// **'The flat pieces, full size. Drag a corner or an edge to reshape them; **R** adds a piece.'**
  String get tourPatternText;

  /// No description provided for @tourGarmentTitle.
  ///
  /// In en, this message translates to:
  /// **'The garment on the body'**
  String get tourGarmentTitle;

  /// No description provided for @tourGarmentText.
  ///
  /// In en, this message translates to:
  /// **'**Simulate** drapes it. Drag to turn around; **⇧-drag** a hem or a seam to change the pattern from here.'**
  String get tourGarmentText;

  /// No description provided for @tourGuideTitle.
  ///
  /// In en, this message translates to:
  /// **'One step at a time'**
  String get tourGuideTitle;

  /// No description provided for @tourGuideText.
  ///
  /// In en, this message translates to:
  /// **'The guide says what to do next. Click a step at the top to jump to it, or close the guide.'**
  String get tourGuideText;

  /// No description provided for @tourHelpTitle.
  ///
  /// In en, this message translates to:
  /// **'Help is always here'**
  String get tourHelpTitle;

  /// No description provided for @tourHelpText.
  ///
  /// In en, this message translates to:
  /// **'**Learn** has short how-tos and shows where things are; **Body** changes the measurements. **⌘Z** undoes anything.'**
  String get tourHelpText;

  /// No description provided for @stepMeasurements.
  ///
  /// In en, this message translates to:
  /// **'Measurements'**
  String get stepMeasurements;

  /// No description provided for @stepMeasurementsGoal.
  ///
  /// In en, this message translates to:
  /// **'Check the body the garment is made for: its size and measurements.'**
  String get stepMeasurementsGoal;

  /// No description provided for @stepMeasurementsAction.
  ///
  /// In en, this message translates to:
  /// **'Open Body'**
  String get stepMeasurementsAction;

  /// No description provided for @stepPattern.
  ///
  /// In en, this message translates to:
  /// **'Pattern'**
  String get stepPattern;

  /// No description provided for @stepPatternGoal.
  ///
  /// In en, this message translates to:
  /// **'Shape the pieces: drag a corner or an edge, or click an edge to type its length.'**
  String get stepPatternGoal;

  /// No description provided for @stepPatternAction.
  ///
  /// In en, this message translates to:
  /// **'Add a piece'**
  String get stepPatternAction;

  /// No description provided for @stepSew.
  ///
  /// In en, this message translates to:
  /// **'Sew'**
  String get stepSew;

  /// No description provided for @stepSewGoal.
  ///
  /// In en, this message translates to:
  /// **'Join the pieces: click an edge on the pattern, then the edge to sew it to.'**
  String get stepSewGoal;

  /// No description provided for @stepSewAction.
  ///
  /// In en, this message translates to:
  /// **'Sew tool'**
  String get stepSewAction;

  /// No description provided for @stepFabric.
  ///
  /// In en, this message translates to:
  /// **'Fabric'**
  String get stepFabric;

  /// No description provided for @stepFabricGoal.
  ///
  /// In en, this message translates to:
  /// **'Choose what the garment is made of, in the panel on the right.'**
  String get stepFabricGoal;

  /// No description provided for @stepDrape.
  ///
  /// In en, this message translates to:
  /// **'Drape'**
  String get stepDrape;

  /// No description provided for @stepDrapeGoal.
  ///
  /// In en, this message translates to:
  /// **'Watch it drape on the body. Then drag the cloth, or ⌥-click to pin it.'**
  String get stepDrapeGoal;

  /// No description provided for @stepOf.
  ///
  /// In en, this message translates to:
  /// **'Step {step} of {total}'**
  String stepOf(int step, int total);

  /// No description provided for @guideNext.
  ///
  /// In en, this message translates to:
  /// **'Next  →'**
  String get guideNext;

  /// No description provided for @guideHide.
  ///
  /// In en, this message translates to:
  /// **'Hide the guide (click a step to bring it back)'**
  String get guideHide;

  /// No description provided for @toolSelect.
  ///
  /// In en, this message translates to:
  /// **'Select'**
  String get toolSelect;

  /// No description provided for @toolSelectHint.
  ///
  /// In en, this message translates to:
  /// **'Click a piece; drag a corner or an edge to reshape it'**
  String get toolSelectHint;

  /// No description provided for @toolRectangle.
  ///
  /// In en, this message translates to:
  /// **'Rectangle'**
  String get toolRectangle;

  /// No description provided for @toolRectangleHint.
  ///
  /// In en, this message translates to:
  /// **'Add a rectangular panel'**
  String get toolRectangleHint;

  /// No description provided for @toolPolygon.
  ///
  /// In en, this message translates to:
  /// **'Polygon'**
  String get toolPolygon;

  /// No description provided for @toolPolygonHint.
  ///
  /// In en, this message translates to:
  /// **'Click the corners of a new piece'**
  String get toolPolygonHint;

  /// No description provided for @toolCurve.
  ///
  /// In en, this message translates to:
  /// **'Curve'**
  String get toolCurve;

  /// No description provided for @toolCurveHint.
  ///
  /// In en, this message translates to:
  /// **'Draw a piece: click for a corner, drag for a smooth point'**
  String get toolCurveHint;

  /// No description provided for @toolEditCurve.
  ///
  /// In en, this message translates to:
  /// **'Edit curves'**
  String get toolEditCurve;

  /// No description provided for @toolEditCurveHint.
  ///
  /// In en, this message translates to:
  /// **'Bend an edge, straighten it, or add a point on it'**
  String get toolEditCurveHint;

  /// No description provided for @toolBend.
  ///
  /// In en, this message translates to:
  /// **'Bend'**
  String get toolBend;

  /// No description provided for @toolBendHint.
  ///
  /// In en, this message translates to:
  /// **'Click an edge, then drag its handles to bend it'**
  String get toolBendHint;

  /// No description provided for @toolStraighten.
  ///
  /// In en, this message translates to:
  /// **'Straighten'**
  String get toolStraighten;

  /// No description provided for @toolStraightenHint.
  ///
  /// In en, this message translates to:
  /// **'Click a curved edge to make it straight'**
  String get toolStraightenHint;

  /// No description provided for @toolAddPoint.
  ///
  /// In en, this message translates to:
  /// **'Add point'**
  String get toolAddPoint;

  /// No description provided for @toolAddPointHint.
  ///
  /// In en, this message translates to:
  /// **'Click an edge where the new point goes'**
  String get toolAddPointHint;

  /// No description provided for @toolCut.
  ///
  /// In en, this message translates to:
  /// **'Cut'**
  String get toolCut;

  /// No description provided for @toolCutHint.
  ///
  /// In en, this message translates to:
  /// **'Draw a line across a piece: it becomes two pieces sewn together'**
  String get toolCutHint;

  /// No description provided for @toolDraw.
  ///
  /// In en, this message translates to:
  /// **'Draw on garment'**
  String get toolDraw;

  /// No description provided for @toolDrawHint.
  ///
  /// In en, this message translates to:
  /// **'Drag on the 3D garment: the line goes on the pattern'**
  String get toolDrawHint;

  /// No description provided for @toolNotch.
  ///
  /// In en, this message translates to:
  /// **'Notch'**
  String get toolNotch;

  /// No description provided for @toolNotchHint.
  ///
  /// In en, this message translates to:
  /// **'Click an edge where the notch goes'**
  String get toolNotchHint;

  /// No description provided for @toolSew.
  ///
  /// In en, this message translates to:
  /// **'Sew'**
  String get toolSew;

  /// No description provided for @toolSewHint.
  ///
  /// In en, this message translates to:
  /// **'Click two edges on the pattern'**
  String get toolSewHint;

  /// No description provided for @viewPattern.
  ///
  /// In en, this message translates to:
  /// **'Pattern'**
  String get viewPattern;

  /// No description provided for @view3d.
  ///
  /// In en, this message translates to:
  /// **'3D'**
  String get view3d;

  /// No description provided for @hintPolygon.
  ///
  /// In en, this message translates to:
  /// **'Polygon: click the corners; click the first one or press Enter to close · Esc cancels · V to stop'**
  String get hintPolygon;

  /// No description provided for @hintCurve.
  ///
  /// In en, this message translates to:
  /// **'Curve: click for a corner, drag for a smooth point; click the first point or press Enter to close · Esc cancels · V to stop'**
  String get hintCurve;

  /// No description provided for @hintBend.
  ///
  /// In en, this message translates to:
  /// **'Bend: click an edge, then drag its handles · V to stop'**
  String get hintBend;

  /// No description provided for @hintStraighten.
  ///
  /// In en, this message translates to:
  /// **'Straighten: click a curved edge · V to stop'**
  String get hintStraighten;

  /// No description provided for @hintAddPoint.
  ///
  /// In en, this message translates to:
  /// **'Add point: click an edge where the point goes · V to stop'**
  String get hintAddPoint;

  /// No description provided for @hintCut.
  ///
  /// In en, this message translates to:
  /// **'Cut: click across a piece, drag for a curve, from outside to outside; Enter cuts · Esc cancels · V to stop'**
  String get hintCut;

  /// No description provided for @hintNotch.
  ///
  /// In en, this message translates to:
  /// **'Notch: click an edge where the notch goes · V to stop'**
  String get hintNotch;

  /// No description provided for @toastPieceAdded.
  ///
  /// In en, this message translates to:
  /// **'Piece added.'**
  String get toastPieceAdded;

  /// No description provided for @toastNotchAdded.
  ///
  /// In en, this message translates to:
  /// **'Notch added.'**
  String get toastNotchAdded;

  /// No description provided for @toastCut.
  ///
  /// In en, this message translates to:
  /// **'Cut in two pieces, sewn together.'**
  String get toastCut;

  /// No description provided for @toastPointAdded.
  ///
  /// In en, this message translates to:
  /// **'Point added.'**
  String get toastPointAdded;

  /// No description provided for @distance.
  ///
  /// In en, this message translates to:
  /// **'Distance'**
  String get distance;

  /// No description provided for @pointPicked.
  ///
  /// In en, this message translates to:
  /// **'Corner picked. ⇧-click another corner to measure the distance between them.'**
  String get pointPicked;

  /// No description provided for @pointsPicked.
  ///
  /// In en, this message translates to:
  /// **'Between the two corners. The second one moves along the line.'**
  String get pointsPicked;

  /// No description provided for @straighten.
  ///
  /// In en, this message translates to:
  /// **'Straighten'**
  String get straighten;

  /// No description provided for @hintSewFirst.
  ///
  /// In en, this message translates to:
  /// **'Sew: click the first edge on the pattern · V to stop'**
  String get hintSewFirst;

  /// No description provided for @hintSewSecond.
  ///
  /// In en, this message translates to:
  /// **'Sew: now click the edge to sew it to · V to stop'**
  String get hintSewSecond;

  /// No description provided for @hintDraw.
  ///
  /// In en, this message translates to:
  /// **'Draw: drag on the 3D garment; the line goes on the pattern · V to stop'**
  String get hintDraw;

  /// No description provided for @hintIdle.
  ///
  /// In en, this message translates to:
  /// **'Pattern: drag a corner or an edge, click an edge for its length · 3D: drag to pull the cloth · ⇧-drag a hem or a seam · ⌘-drag to move a piece · ⌥-click to pin'**
  String get hintIdle;

  /// No description provided for @hintSelection.
  ///
  /// In en, this message translates to:
  /// **'Adjust the selection in the inspector · ⌫ deletes · ⌘Z undoes'**
  String get hintSelection;

  /// No description provided for @statsSim.
  ///
  /// In en, this message translates to:
  /// **'Cloth {points} points · {backend} step {stepMs} ms · {time} s · '**
  String statsSim(int points, String backend, String stepMs, String time);

  /// No description provided for @statsRender.
  ///
  /// In en, this message translates to:
  /// **'Rust render · pattern {boardMs} ms · 3D {garmentMs} ms · {fps} fps'**
  String statsRender(String boardMs, String garmentMs, String fps);

  /// No description provided for @toastSewn.
  ///
  /// In en, this message translates to:
  /// **'Sewn. Flip it in the inspector if it looks twisted.'**
  String get toastSewn;

  /// No description provided for @toastPinned.
  ///
  /// In en, this message translates to:
  /// **'Pinned. Remove it in the inspector.'**
  String get toastPinned;

  /// No description provided for @toastLineAdded.
  ///
  /// In en, this message translates to:
  /// **'Line added to the pattern.'**
  String get toastLineAdded;

  /// No description provided for @deleteTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete “{name}”?'**
  String deleteTitle(String name);

  /// No description provided for @deleteUndo.
  ///
  /// In en, this message translates to:
  /// **'You can undo this with ⌘Z.'**
  String get deleteUndo;

  /// No description provided for @camFront.
  ///
  /// In en, this message translates to:
  /// **'Front'**
  String get camFront;

  /// No description provided for @camSide.
  ///
  /// In en, this message translates to:
  /// **'Side'**
  String get camSide;

  /// No description provided for @camBack.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get camBack;

  /// No description provided for @camFrontTip.
  ///
  /// In en, this message translates to:
  /// **'View from the front  1'**
  String get camFrontTip;

  /// No description provided for @camSideTip.
  ///
  /// In en, this message translates to:
  /// **'View from the side  2'**
  String get camSideTip;

  /// No description provided for @camBackTip.
  ///
  /// In en, this message translates to:
  /// **'View from the back  3'**
  String get camBackTip;

  /// No description provided for @camThreeQuarterTip.
  ///
  /// In en, this message translates to:
  /// **'View from the three-quarter angle  4'**
  String get camThreeQuarterTip;

  /// No description provided for @simulate.
  ///
  /// In en, this message translates to:
  /// **'Simulate'**
  String get simulate;

  /// No description provided for @pause.
  ///
  /// In en, this message translates to:
  /// **'Pause'**
  String get pause;

  /// No description provided for @simulateTip.
  ///
  /// In en, this message translates to:
  /// **'Drape the garment  Space'**
  String get simulateTip;

  /// No description provided for @pauseTip.
  ///
  /// In en, this message translates to:
  /// **'Pause  Space'**
  String get pauseTip;

  /// No description provided for @restartTip.
  ///
  /// In en, this message translates to:
  /// **'Start again from the pattern'**
  String get restartTip;

  /// No description provided for @maximizeViewTip.
  ///
  /// In en, this message translates to:
  /// **'Use the space of both views  M'**
  String get maximizeViewTip;

  /// No description provided for @restoreViewsTip.
  ///
  /// In en, this message translates to:
  /// **'Show both views again  M'**
  String get restoreViewsTip;

  /// No description provided for @inspector.
  ///
  /// In en, this message translates to:
  /// **'Inspector'**
  String get inspector;

  /// No description provided for @selectPanel.
  ///
  /// In en, this message translates to:
  /// **'Select a panel'**
  String get selectPanel;

  /// No description provided for @selectPanelHint.
  ///
  /// In en, this message translates to:
  /// **'Click a piece on the pattern or the garment\nto adjust it here. {count, plural, =1{1 piece} other{{count} pieces}} in this project.'**
  String selectPanelHint(int count);

  /// No description provided for @arrange.
  ///
  /// In en, this message translates to:
  /// **'Arrange around the body'**
  String get arrange;

  /// No description provided for @arrangeHint.
  ///
  /// In en, this message translates to:
  /// **'Places every piece around the body in seam order, at the waist.'**
  String get arrangeHint;

  /// No description provided for @pins.
  ///
  /// In en, this message translates to:
  /// **'Pins'**
  String get pins;

  /// No description provided for @pinsEmpty.
  ///
  /// In en, this message translates to:
  /// **'⌥-click the cloth in 3D to hold it in place.'**
  String get pinsEmpty;

  /// No description provided for @pinsAdd.
  ///
  /// In en, this message translates to:
  /// **'⌥-click the cloth in 3D to add one.'**
  String get pinsAdd;

  /// No description provided for @removePin.
  ///
  /// In en, this message translates to:
  /// **'Remove this pin'**
  String get removePin;

  /// No description provided for @wholeGarment.
  ///
  /// In en, this message translates to:
  /// **'Whole garment'**
  String get wholeGarment;

  /// No description provided for @piecesSelected.
  ///
  /// In en, this message translates to:
  /// **'{count} pieces selected. Click one piece on the pattern to edit it.'**
  String piecesSelected(int count);

  /// No description provided for @piece.
  ///
  /// In en, this message translates to:
  /// **'Piece'**
  String get piece;

  /// No description provided for @mirrorCopy.
  ///
  /// In en, this message translates to:
  /// **'Mirror copy — follows its source'**
  String get mirrorCopy;

  /// No description provided for @unlink.
  ///
  /// In en, this message translates to:
  /// **'Unlink'**
  String get unlink;

  /// No description provided for @area.
  ///
  /// In en, this message translates to:
  /// **'Area'**
  String get area;

  /// No description provided for @perimeter.
  ///
  /// In en, this message translates to:
  /// **'Perimeter'**
  String get perimeter;

  /// No description provided for @seamAllowance.
  ///
  /// In en, this message translates to:
  /// **'Seam allowance'**
  String get seamAllowance;

  /// No description provided for @grainline.
  ///
  /// In en, this message translates to:
  /// **'Grainline'**
  String get grainline;

  /// No description provided for @mirror.
  ///
  /// In en, this message translates to:
  /// **'Mirror'**
  String get mirror;

  /// No description provided for @onTheBody.
  ///
  /// In en, this message translates to:
  /// **'On the body'**
  String get onTheBody;

  /// No description provided for @notSimulated.
  ///
  /// In en, this message translates to:
  /// **'Not simulated yet: place it to see it in 3D.'**
  String get notSimulated;

  /// No description provided for @placeOnBody.
  ///
  /// In en, this message translates to:
  /// **'Place on the body'**
  String get placeOnBody;

  /// No description provided for @aroundBody.
  ///
  /// In en, this message translates to:
  /// **'Body'**
  String get aroundBody;

  /// No description provided for @aroundLeftArm.
  ///
  /// In en, this message translates to:
  /// **'Left arm'**
  String get aroundLeftArm;

  /// No description provided for @aroundRightArm.
  ///
  /// In en, this message translates to:
  /// **'Right arm'**
  String get aroundRightArm;

  /// No description provided for @aroundLeftLeg.
  ///
  /// In en, this message translates to:
  /// **'Left leg'**
  String get aroundLeftLeg;

  /// No description provided for @aroundRightLeg.
  ///
  /// In en, this message translates to:
  /// **'Right leg'**
  String get aroundRightLeg;

  /// No description provided for @aroundFlat.
  ///
  /// In en, this message translates to:
  /// **'Flat'**
  String get aroundFlat;

  /// No description provided for @aroundAngle.
  ///
  /// In en, this message translates to:
  /// **'Around (0 = front)'**
  String get aroundAngle;

  /// No description provided for @aroundArmAngle.
  ///
  /// In en, this message translates to:
  /// **'Around the arm (0 = outside)'**
  String get aroundArmAngle;

  /// No description provided for @topHeight.
  ///
  /// In en, this message translates to:
  /// **'Top height'**
  String get topHeight;

  /// No description provided for @fabric.
  ///
  /// In en, this message translates to:
  /// **'Fabric'**
  String get fabric;

  /// No description provided for @garments.
  ///
  /// In en, this message translates to:
  /// **'Garments'**
  String get garments;

  /// No description provided for @garmentsHint.
  ///
  /// In en, this message translates to:
  /// **'Worn from the body out: a garment drapes over the ones above it in the list.'**
  String get garmentsHint;

  /// No description provided for @garmentNextToBody.
  ///
  /// In en, this message translates to:
  /// **'Next to the body'**
  String get garmentNextToBody;

  /// No description provided for @garmentOver.
  ///
  /// In en, this message translates to:
  /// **'Worn over the one above'**
  String get garmentOver;

  /// No description provided for @garmentMoveIn.
  ///
  /// In en, this message translates to:
  /// **'Wear it under'**
  String get garmentMoveIn;

  /// No description provided for @garmentMoveOut.
  ///
  /// In en, this message translates to:
  /// **'Wear it over'**
  String get garmentMoveOut;

  /// No description provided for @garmentRemove.
  ///
  /// In en, this message translates to:
  /// **'Delete the garment and its pieces'**
  String get garmentRemove;

  /// No description provided for @addGarment.
  ///
  /// In en, this message translates to:
  /// **'Add a garment'**
  String get addGarment;

  /// No description provided for @garmentNewName.
  ///
  /// In en, this message translates to:
  /// **'Garment {number}'**
  String garmentNewName(int number);

  /// No description provided for @pieceGarment.
  ///
  /// In en, this message translates to:
  /// **'Garment'**
  String get pieceGarment;

  /// No description provided for @fabricCotton.
  ///
  /// In en, this message translates to:
  /// **'Cotton'**
  String get fabricCotton;

  /// No description provided for @fabricCottonBlurb.
  ///
  /// In en, this message translates to:
  /// **'Crisp and breathable'**
  String get fabricCottonBlurb;

  /// No description provided for @fabricDenim.
  ///
  /// In en, this message translates to:
  /// **'Denim'**
  String get fabricDenim;

  /// No description provided for @fabricDenimBlurb.
  ///
  /// In en, this message translates to:
  /// **'Heavy, holds its shape'**
  String get fabricDenimBlurb;

  /// No description provided for @fabricJersey.
  ///
  /// In en, this message translates to:
  /// **'Jersey'**
  String get fabricJersey;

  /// No description provided for @fabricJerseyBlurb.
  ///
  /// In en, this message translates to:
  /// **'Stretchy knit'**
  String get fabricJerseyBlurb;

  /// No description provided for @fabricSilk.
  ///
  /// In en, this message translates to:
  /// **'Silk'**
  String get fabricSilk;

  /// No description provided for @fabricSilkBlurb.
  ///
  /// In en, this message translates to:
  /// **'Light and fluid'**
  String get fabricSilkBlurb;

  /// No description provided for @fabricWool.
  ///
  /// In en, this message translates to:
  /// **'Wool'**
  String get fabricWool;

  /// No description provided for @fabricWoolBlurb.
  ///
  /// In en, this message translates to:
  /// **'Soft, with body'**
  String get fabricWoolBlurb;

  /// No description provided for @edgeLength.
  ///
  /// In en, this message translates to:
  /// **'Edge · {length} cm'**
  String edgeLength(String length);

  /// No description provided for @edgeOpen.
  ///
  /// In en, this message translates to:
  /// **'Open edge (a hem or an opening). In 3D, ⇧-drag it to lengthen the garment.'**
  String get edgeOpen;

  /// No description provided for @edgeLengthLabel.
  ///
  /// In en, this message translates to:
  /// **'Length'**
  String get edgeLengthLabel;

  /// No description provided for @edgeSewn.
  ///
  /// In en, this message translates to:
  /// **'Sewn to {piece} · {mismatch} mm difference. Its length changes on both sides. In 3D, ⇧-drag it to take it in.'**
  String edgeSewn(String piece, String mismatch);

  /// No description provided for @matchOtherSide.
  ///
  /// In en, this message translates to:
  /// **'Make the other side this length'**
  String get matchOtherSide;

  /// No description provided for @gathered.
  ///
  /// In en, this message translates to:
  /// **'Gathered'**
  String get gathered;

  /// No description provided for @gatheredHint.
  ///
  /// In en, this message translates to:
  /// **'The longer side is gathered evenly onto the shorter one ({ratio} : 1).'**
  String gatheredHint(String ratio);

  /// No description provided for @flipSeam.
  ///
  /// In en, this message translates to:
  /// **'Flip seam'**
  String get flipSeam;

  /// No description provided for @unpick.
  ///
  /// In en, this message translates to:
  /// **'Unpick'**
  String get unpick;

  /// No description provided for @changeTakenIn.
  ///
  /// In en, this message translates to:
  /// **'Taken in {amount} mm'**
  String changeTakenIn(String amount);

  /// No description provided for @changeLetOut.
  ///
  /// In en, this message translates to:
  /// **'Let out {amount} mm'**
  String changeLetOut(String amount);

  /// No description provided for @changeLonger.
  ///
  /// In en, this message translates to:
  /// **'Longer {amount} mm'**
  String changeLonger(String amount);

  /// No description provided for @changeMoved.
  ///
  /// In en, this message translates to:
  /// **'Moved {amount} mm'**
  String changeMoved(String amount);

  /// No description provided for @changeShorter.
  ///
  /// In en, this message translates to:
  /// **'Shorter {amount} mm'**
  String changeShorter(String amount);

  /// No description provided for @learnIntro.
  ///
  /// In en, this message translates to:
  /// **'Short how-tos. “Show me” points at where it happens; your project is not changed.'**
  String get learnIntro;

  /// No description provided for @showMe.
  ///
  /// In en, this message translates to:
  /// **'Show me'**
  String get showMe;

  /// No description provided for @learnDrape.
  ///
  /// In en, this message translates to:
  /// **'Drape the garment'**
  String get learnDrape;

  /// No description provided for @learnDrapeHow.
  ///
  /// In en, this message translates to:
  /// **'Press **Simulate** (**Space**): the pieces close their seams and fall onto the body.'**
  String get learnDrapeHow;

  /// No description provided for @learnFabric.
  ///
  /// In en, this message translates to:
  /// **'Try another fabric'**
  String get learnFabric;

  /// No description provided for @learnFabricHow.
  ///
  /// In en, this message translates to:
  /// **'With nothing selected, pick a swatch under **Fabric**. Fabric changes how the garment hangs.'**
  String get learnFabricHow;

  /// No description provided for @learnLength.
  ///
  /// In en, this message translates to:
  /// **'Change a length'**
  String get learnLength;

  /// No description provided for @learnLengthHow.
  ///
  /// In en, this message translates to:
  /// **'Click an edge (a side seam to lengthen, a hem to widen), then type its **Length** or drag the slider. Or hold **⇧** and drag the hem on the garment.'**
  String get learnLengthHow;

  /// No description provided for @learnFit.
  ///
  /// In en, this message translates to:
  /// **'Take in a seam'**
  String get learnFit;

  /// No description provided for @learnFitHow.
  ///
  /// In en, this message translates to:
  /// **'Hold **⇧** and drag a seam on the garment: towards the body takes it in, away lets it out.'**
  String get learnFitHow;

  /// No description provided for @learnSew.
  ///
  /// In en, this message translates to:
  /// **'Sew two pieces'**
  String get learnSew;

  /// No description provided for @learnSewHow.
  ///
  /// In en, this message translates to:
  /// **'With **Sew** (**S**), click an edge on the pattern, then the edge to sew it to.'**
  String get learnSewHow;

  /// No description provided for @learnDraw.
  ///
  /// In en, this message translates to:
  /// **'Draw a style line'**
  String get learnDraw;

  /// No description provided for @learnDrawHow.
  ///
  /// In en, this message translates to:
  /// **'With **Draw on garment** (**D**), drag across the 3D garment: the line lands on the pattern.'**
  String get learnDrawHow;

  /// No description provided for @learnMove.
  ///
  /// In en, this message translates to:
  /// **'Move a piece around the body'**
  String get learnMove;

  /// No description provided for @learnMoveHow.
  ///
  /// In en, this message translates to:
  /// **'Hold **⌘** and drag a piece on the garment, or set it in the inspector.'**
  String get learnMoveHow;

  /// No description provided for @learnPin.
  ///
  /// In en, this message translates to:
  /// **'Pull and pin the cloth'**
  String get learnPin;

  /// No description provided for @learnPinHow.
  ///
  /// In en, this message translates to:
  /// **'Drag the cloth in 3D to pull it. **⌥-click** pins it where it is.'**
  String get learnPinHow;

  /// No description provided for @learnLook.
  ///
  /// In en, this message translates to:
  /// **'Look around'**
  String get learnLook;

  /// No description provided for @learnLookHow.
  ///
  /// In en, this message translates to:
  /// **'Drag to turn around the body; scroll or pinch to zoom. Keys **1**–**4**: front, side, back, three quarters.'**
  String get learnLookHow;

  /// No description provided for @learnBody.
  ///
  /// In en, this message translates to:
  /// **'Dress another body'**
  String get learnBody;

  /// No description provided for @learnBodyHow.
  ///
  /// In en, this message translates to:
  /// **'Open **Body**: choose a size or enter measurements, and the garment drapes on it.'**
  String get learnBodyHow;

  /// No description provided for @learnDrapeStop.
  ///
  /// In en, this message translates to:
  /// **'Simulate'**
  String get learnDrapeStop;

  /// No description provided for @learnDrapeStopText.
  ///
  /// In en, this message translates to:
  /// **'Press **Simulate** (**Space**) to drape the garment, again to pause. The arrow next to it starts again from the pattern.'**
  String get learnDrapeStopText;

  /// No description provided for @learnFabricStop.
  ///
  /// In en, this message translates to:
  /// **'Fabric'**
  String get learnFabricStop;

  /// No description provided for @learnFabricStopText.
  ///
  /// In en, this message translates to:
  /// **'With nothing selected, the panel on the right shows the garment: pick a swatch under **Fabric**. The drape follows.'**
  String get learnFabricStopText;

  /// No description provided for @learnPickEdge.
  ///
  /// In en, this message translates to:
  /// **'Pick an edge'**
  String get learnPickEdge;

  /// No description provided for @learnPickEdgeText.
  ///
  /// In en, this message translates to:
  /// **'Click an edge on the pattern, or on the garment.'**
  String get learnPickEdgeText;

  /// No description provided for @learnEdgeLength.
  ///
  /// In en, this message translates to:
  /// **'Its length'**
  String get learnEdgeLength;

  /// No description provided for @learnEdgeLengthText.
  ///
  /// In en, this message translates to:
  /// **'Type the new **Length** and press **Enter**, or drag the slider. A sewn edge’s other side follows, so seams stay matched.'**
  String get learnEdgeLengthText;

  /// No description provided for @learnDragHem.
  ///
  /// In en, this message translates to:
  /// **'Or drag it'**
  String get learnDragHem;

  /// No description provided for @learnDragHemText.
  ///
  /// In en, this message translates to:
  /// **'Hold **⇧** and drag the hem on the garment: down lengthens, up shortens.'**
  String get learnDragHemText;

  /// No description provided for @learnDragSeam.
  ///
  /// In en, this message translates to:
  /// **'Or drag it'**
  String get learnDragSeam;

  /// No description provided for @learnDragSeamText.
  ///
  /// In en, this message translates to:
  /// **'Hold **⇧** and drag a seam on the garment: towards the body takes it in, away lets it out. Near one end, only that end moves.'**
  String get learnDragSeamText;

  /// No description provided for @learnSewTool.
  ///
  /// In en, this message translates to:
  /// **'Sew tool'**
  String get learnSewTool;

  /// No description provided for @learnSewToolText.
  ///
  /// In en, this message translates to:
  /// **'Pick **Sew** here, or press **S**.'**
  String get learnSewToolText;

  /// No description provided for @learnTwoEdges.
  ///
  /// In en, this message translates to:
  /// **'Click two edges'**
  String get learnTwoEdges;

  /// No description provided for @learnTwoEdgesText.
  ///
  /// In en, this message translates to:
  /// **'Click an edge, then the edge to sew it to. Matching arrows show how they join.'**
  String get learnTwoEdgesText;

  /// No description provided for @learnDrawTool.
  ///
  /// In en, this message translates to:
  /// **'Draw on garment'**
  String get learnDrawTool;

  /// No description provided for @learnDrawToolText.
  ///
  /// In en, this message translates to:
  /// **'Pick **Draw on garment** here, or press **D**.'**
  String get learnDrawToolText;

  /// No description provided for @learnDrawAcross.
  ///
  /// In en, this message translates to:
  /// **'Drag across the garment'**
  String get learnDrawAcross;

  /// No description provided for @learnDrawAcrossText.
  ///
  /// In en, this message translates to:
  /// **'The line lands on the pattern, for a yoke or a pocket.'**
  String get learnDrawAcrossText;

  /// No description provided for @learnDragPiece.
  ///
  /// In en, this message translates to:
  /// **'Drag a piece'**
  String get learnDragPiece;

  /// No description provided for @learnDragPieceText.
  ///
  /// In en, this message translates to:
  /// **'Hold **⌘** and drag a piece: sideways turns it around the body, up and down changes its height.'**
  String get learnDragPieceText;

  /// No description provided for @learnSetPlacement.
  ///
  /// In en, this message translates to:
  /// **'Or set it here'**
  String get learnSetPlacement;

  /// No description provided for @learnSetPlacementText.
  ///
  /// In en, this message translates to:
  /// **'Select the piece: **On the body** has the same settings, as numbers you can type.'**
  String get learnSetPlacementText;

  /// No description provided for @learnPullPin.
  ///
  /// In en, this message translates to:
  /// **'Pull and pin'**
  String get learnPullPin;

  /// No description provided for @learnPullPinText.
  ///
  /// In en, this message translates to:
  /// **'Drag the cloth to pull it. **⌥-click** pins it where it is.'**
  String get learnPullPinText;

  /// No description provided for @learnYourPins.
  ///
  /// In en, this message translates to:
  /// **'Your pins'**
  String get learnYourPins;

  /// No description provided for @learnYourPinsText.
  ///
  /// In en, this message translates to:
  /// **'With nothing selected, pins are listed under **Pins**; **×** removes one.'**
  String get learnYourPinsText;

  /// No description provided for @learnViews.
  ///
  /// In en, this message translates to:
  /// **'Views'**
  String get learnViews;

  /// No description provided for @learnViewsText.
  ///
  /// In en, this message translates to:
  /// **'**Front**, **Side**, **Back** and **¾**, or the keys **1**–**4**.'**
  String get learnViewsText;

  /// No description provided for @learnTurnZoom.
  ///
  /// In en, this message translates to:
  /// **'Turn and zoom'**
  String get learnTurnZoom;

  /// No description provided for @learnTurnZoomText.
  ///
  /// In en, this message translates to:
  /// **'Drag around the body to turn; scroll, or pinch on a trackpad, to zoom.'**
  String get learnTurnZoomText;

  /// No description provided for @learnBodyStop.
  ///
  /// In en, this message translates to:
  /// **'Body'**
  String get learnBodyStop;

  /// No description provided for @learnBodyStopText.
  ///
  /// In en, this message translates to:
  /// **'Choose a size or enter measurements: the body is refitted and the garment drapes on it.'**
  String get learnBodyStopText;

  /// No description provided for @errNotFound.
  ///
  /// In en, this message translates to:
  /// **'{id} does not exist.'**
  String errNotFound(String id);

  /// No description provided for @errLinkedMirror.
  ///
  /// In en, this message translates to:
  /// **'{panel} is a linked mirror copy: edit its source, or unlink it first.'**
  String errLinkedMirror(String panel);

  /// No description provided for @errSewn.
  ///
  /// In en, this message translates to:
  /// **'This edge is sewn: unpick the seam first.'**
  String get errSewn;

  /// No description provided for @errNothingToUndo.
  ///
  /// In en, this message translates to:
  /// **'Nothing to undo.'**
  String get errNothingToUndo;

  /// No description provided for @errNothingToRedo.
  ///
  /// In en, this message translates to:
  /// **'Nothing to redo.'**
  String get errNothingToRedo;

  /// No description provided for @errNothingToSimulate.
  ///
  /// In en, this message translates to:
  /// **'Nothing to simulate: place at least one piece on the body.'**
  String get errNothingToSimulate;

  /// No description provided for @errInverted.
  ///
  /// In en, this message translates to:
  /// **'That would turn a piece inside out.'**
  String get errInverted;

  /// No description provided for @errRange.
  ///
  /// In en, this message translates to:
  /// **'The value must be between {min} and {max}.'**
  String errRange(String min, String max);

  /// No description provided for @errNotPositive.
  ///
  /// In en, this message translates to:
  /// **'The value must be greater than zero.'**
  String get errNotPositive;

  /// No description provided for @errNegative.
  ///
  /// In en, this message translates to:
  /// **'The value must not be negative.'**
  String get errNegative;

  /// No description provided for @errEmpty.
  ///
  /// In en, this message translates to:
  /// **'The name must not be empty.'**
  String get errEmpty;

  /// No description provided for @errNotSimple.
  ///
  /// In en, this message translates to:
  /// **'The outline must not cross itself.'**
  String get errNotSimple;

  /// No description provided for @errNotAcross.
  ///
  /// In en, this message translates to:
  /// **'The cut must cross one piece from side to side: start and end outside it.'**
  String get errNotAcross;

  /// No description provided for @errMirrored.
  ///
  /// In en, this message translates to:
  /// **'This piece has a linked mirror copy: unlink the copy first.'**
  String get errMirrored;

  /// No description provided for @errLastGarment.
  ///
  /// In en, this message translates to:
  /// **'A project keeps at least one garment.'**
  String get errLastGarment;

  /// No description provided for @errFileIo.
  ///
  /// In en, this message translates to:
  /// **'Could not read or write the project files.'**
  String get errFileIo;

  /// No description provided for @errImportMalformed.
  ///
  /// In en, this message translates to:
  /// **'This file cannot be read as {format}.'**
  String errImportMalformed(String format);

  /// No description provided for @errImportBinaryDxf.
  ///
  /// In en, this message translates to:
  /// **'Binary DXF files cannot be read: save the file as ASCII DXF.'**
  String get errImportBinaryDxf;

  /// No description provided for @errImportNoPieces.
  ///
  /// In en, this message translates to:
  /// **'No pattern piece found: the drawing has no closed outline.'**
  String get errImportNoPieces;

  /// No description provided for @errImportPagesDiffer.
  ///
  /// In en, this message translates to:
  /// **'The pages of this PDF do not repeat one drawing, so they cannot be put back together.'**
  String get errImportPagesDiffer;

  /// No description provided for @errImportEncrypted.
  ///
  /// In en, this message translates to:
  /// **'This PDF is protected and cannot be read.'**
  String get errImportEncrypted;

  /// No description provided for @errReadFile.
  ///
  /// In en, this message translates to:
  /// **'Could not read this file.'**
  String get errReadFile;

  /// No description provided for @errNotAProject.
  ///
  /// In en, this message translates to:
  /// **'This folder is not a Rustle3D project.'**
  String get errNotAProject;

  /// No description provided for @errTooNew.
  ///
  /// In en, this message translates to:
  /// **'This project was made by a newer version of Rustle3D.'**
  String get errTooNew;

  /// No description provided for @errCorrupted.
  ///
  /// In en, this message translates to:
  /// **'This project file is damaged.'**
  String get errCorrupted;

  /// No description provided for @errAlreadyExists.
  ///
  /// In en, this message translates to:
  /// **'A project already exists there.'**
  String get errAlreadyExists;

  /// No description provided for @errArchiveInvalid.
  ///
  /// In en, this message translates to:
  /// **'This file is not a readable .zip archive.'**
  String get errArchiveInvalid;

  /// No description provided for @errArchiveNoProject.
  ///
  /// In en, this message translates to:
  /// **'This archive holds no Rustle3D project.'**
  String get errArchiveNoProject;

  /// No description provided for @errArchiveTooLarge.
  ///
  /// In en, this message translates to:
  /// **'This archive is too large to import (more than {limit} MB once unpacked).'**
  String errArchiveTooLarge(String limit);

  /// No description provided for @photo.
  ///
  /// In en, this message translates to:
  /// **'Photo'**
  String get photo;

  /// No description provided for @photoTip.
  ///
  /// In en, this message translates to:
  /// **'Render a photorealistic image of the garment'**
  String get photoTip;

  /// No description provided for @photoTitle.
  ///
  /// In en, this message translates to:
  /// **'Render a photo'**
  String get photoTitle;

  /// No description provided for @photoIntro.
  ///
  /// In en, this message translates to:
  /// **'A photorealistic image of the garment as the 3D view shows it now, in a photo studio.'**
  String get photoIntro;

  /// No description provided for @photoQuality.
  ///
  /// In en, this message translates to:
  /// **'Quality'**
  String get photoQuality;

  /// No description provided for @photoQuick.
  ///
  /// In en, this message translates to:
  /// **'Quick'**
  String get photoQuick;

  /// No description provided for @photoQuickHint.
  ///
  /// In en, this message translates to:
  /// **'A first look in seconds'**
  String get photoQuickHint;

  /// No description provided for @photoGood.
  ///
  /// In en, this message translates to:
  /// **'Good'**
  String get photoGood;

  /// No description provided for @photoGoodHint.
  ///
  /// In en, this message translates to:
  /// **'Clean enough to show'**
  String get photoGoodHint;

  /// No description provided for @photoBest.
  ///
  /// In en, this message translates to:
  /// **'Best'**
  String get photoBest;

  /// No description provided for @photoBestHint.
  ///
  /// In en, this message translates to:
  /// **'For print and close-ups'**
  String get photoBestHint;

  /// No description provided for @photoLighting.
  ///
  /// In en, this message translates to:
  /// **'Lighting'**
  String get photoLighting;

  /// No description provided for @lightStudio.
  ///
  /// In en, this message translates to:
  /// **'Studio'**
  String get lightStudio;

  /// No description provided for @lightStudioHint.
  ///
  /// In en, this message translates to:
  /// **'Key, fill and rim lights on a light grey backdrop'**
  String get lightStudioHint;

  /// No description provided for @lightSoft.
  ///
  /// In en, this message translates to:
  /// **'Soft'**
  String get lightSoft;

  /// No description provided for @lightSoftHint.
  ///
  /// In en, this message translates to:
  /// **'Large soft lights on a white backdrop'**
  String get lightSoftHint;

  /// No description provided for @lightDramatic.
  ///
  /// In en, this message translates to:
  /// **'Dramatic'**
  String get lightDramatic;

  /// No description provided for @lightDramaticHint.
  ///
  /// In en, this message translates to:
  /// **'One hard light on a dark backdrop'**
  String get lightDramaticHint;

  /// No description provided for @photoSize.
  ///
  /// In en, this message translates to:
  /// **'Size'**
  String get photoSize;

  /// No description provided for @photoSizeView.
  ///
  /// In en, this message translates to:
  /// **'As the 3D view'**
  String get photoSizeView;

  /// No description provided for @photoAdvanced.
  ///
  /// In en, this message translates to:
  /// **'Advanced'**
  String get photoAdvanced;

  /// No description provided for @photoSamples.
  ///
  /// In en, this message translates to:
  /// **'Samples per pixel'**
  String get photoSamples;

  /// No description provided for @photoBounces.
  ///
  /// In en, this message translates to:
  /// **'Light bounces'**
  String get photoBounces;

  /// No description provided for @photoDenoise.
  ///
  /// In en, this message translates to:
  /// **'Remove noise'**
  String get photoDenoise;

  /// No description provided for @photoRender.
  ///
  /// In en, this message translates to:
  /// **'Render'**
  String get photoRender;

  /// No description provided for @photoRendering.
  ///
  /// In en, this message translates to:
  /// **'Rendering… {done} / {total} samples · {seconds} s'**
  String photoRendering(int done, int total, String seconds);

  /// No description provided for @photoDone.
  ///
  /// In en, this message translates to:
  /// **'Done in {seconds} s'**
  String photoDone(String seconds);

  /// No description provided for @photoStopped.
  ///
  /// In en, this message translates to:
  /// **'Stopped at {done} samples'**
  String photoStopped(int done);

  /// No description provided for @photoFailed.
  ///
  /// In en, this message translates to:
  /// **'The photo could not be rendered: {reason}'**
  String photoFailed(String reason);

  /// No description provided for @photoStop.
  ///
  /// In en, this message translates to:
  /// **'Stop'**
  String get photoStop;

  /// No description provided for @photoStopTip.
  ///
  /// In en, this message translates to:
  /// **'Stop refining and keep the image as it is'**
  String get photoStopTip;

  /// No description provided for @photoSave.
  ///
  /// In en, this message translates to:
  /// **'Save image…'**
  String get photoSave;

  /// No description provided for @photoClose.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get photoClose;

  /// No description provided for @photoCloseTip.
  ///
  /// In en, this message translates to:
  /// **'Back to the live 3D view'**
  String get photoCloseTip;

  /// No description provided for @photoSaved.
  ///
  /// In en, this message translates to:
  /// **'Saved {file}.'**
  String photoSaved(String file);

  /// No description provided for @errPhotoTooLarge.
  ///
  /// In en, this message translates to:
  /// **'This image is too large. Choose a smaller size (at most 3840 × 2160).'**
  String get errPhotoTooLarge;

  /// No description provided for @errNoPhoto.
  ///
  /// In en, this message translates to:
  /// **'There is no photo to save.'**
  String get errNoPhoto;

  /// No description provided for @learnPhoto.
  ///
  /// In en, this message translates to:
  /// **'Take a photo'**
  String get learnPhoto;

  /// No description provided for @learnPhotoHow.
  ///
  /// In en, this message translates to:
  /// **'A photorealistic image of the garment, to show a client or publish.'**
  String get learnPhotoHow;

  /// No description provided for @learnPhotoStop.
  ///
  /// In en, this message translates to:
  /// **'Photo'**
  String get learnPhotoStop;

  /// No description provided for @learnPhotoStopText.
  ///
  /// In en, this message translates to:
  /// **'Turn the 3D view to the angle you like, click the **camera** button, choose the quality and the lighting, then **Render**. The image gets cleaner in the 3D view; **Save image…** keeps it as a PNG file.'**
  String get learnPhotoStopText;

  /// No description provided for @sizes.
  ///
  /// In en, this message translates to:
  /// **'Sizes'**
  String get sizes;

  /// No description provided for @sizesHint.
  ///
  /// In en, this message translates to:
  /// **'Grade the pattern to a range of sizes. It stays drawn in the base size; the other sizes\' outlines nest around each piece.'**
  String get sizesHint;

  /// No description provided for @addSizes.
  ///
  /// In en, this message translates to:
  /// **'Add a size range'**
  String get addSizes;

  /// No description provided for @sizeChart.
  ///
  /// In en, this message translates to:
  /// **'Size chart'**
  String get sizeChart;

  /// No description provided for @chartWomen.
  ///
  /// In en, this message translates to:
  /// **'Women'**
  String get chartWomen;

  /// No description provided for @chartMen.
  ///
  /// In en, this message translates to:
  /// **'Men'**
  String get chartMen;

  /// No description provided for @sizeFrom.
  ///
  /// In en, this message translates to:
  /// **'From'**
  String get sizeFrom;

  /// No description provided for @sizeTo.
  ///
  /// In en, this message translates to:
  /// **'To'**
  String get sizeTo;

  /// No description provided for @sizeBase.
  ///
  /// In en, this message translates to:
  /// **'Base size'**
  String get sizeBase;

  /// No description provided for @sizesDialogHint.
  ///
  /// In en, this message translates to:
  /// **'Pieces added from a template afterwards are graded to every size. Each size is draped on a body with its measurements.'**
  String get sizesDialogHint;

  /// No description provided for @sizeShown.
  ///
  /// In en, this message translates to:
  /// **'Shown in 3D'**
  String get sizeShown;

  /// No description provided for @sizesChange.
  ///
  /// In en, this message translates to:
  /// **'Change'**
  String get sizesChange;

  /// No description provided for @sizesRemove.
  ///
  /// In en, this message translates to:
  /// **'Remove the sizes'**
  String get sizesRemove;

  /// No description provided for @sizeBaseTag.
  ///
  /// In en, this message translates to:
  /// **'base'**
  String get sizeBaseTag;

  /// No description provided for @gradeStep.
  ///
  /// In en, this message translates to:
  /// **'Grade per size'**
  String get gradeStep;

  /// No description provided for @gradeStepHint.
  ///
  /// In en, this message translates to:
  /// **'How far this point moves from one size to the next.'**
  String get gradeStepHint;

  /// No description provided for @gradeCustom.
  ///
  /// In en, this message translates to:
  /// **'Set per size (by a template or an agent). A step typed here replaces it.'**
  String get gradeCustom;

  /// No description provided for @errNoSizes.
  ///
  /// In en, this message translates to:
  /// **'Add a size range first.'**
  String get errNoSizes;

  /// No description provided for @errUnknownSize.
  ///
  /// In en, this message translates to:
  /// **'That size is not in the range.'**
  String get errUnknownSize;

  /// No description provided for @errTooFewSizes.
  ///
  /// In en, this message translates to:
  /// **'A size range needs at least two sizes.'**
  String get errTooFewSizes;

  /// No description provided for @errDuplicateName.
  ///
  /// In en, this message translates to:
  /// **'Two sizes have the same name.'**
  String get errDuplicateName;

  /// No description provided for @errBodyOutOfRange.
  ///
  /// In en, this message translates to:
  /// **'A size of this range would give the avatar measurements outside what it accepts.'**
  String get errBodyOutOfRange;

  /// No description provided for @errBaseOffset.
  ///
  /// In en, this message translates to:
  /// **'The base size cannot move: it is the pattern as drawn.'**
  String get errBaseOffset;

  /// No description provided for @sizesOnBoard.
  ///
  /// In en, this message translates to:
  /// **'Show the sizes on the board'**
  String get sizesOnBoard;
}

class _LDelegate extends LocalizationsDelegate<L> {
  const _LDelegate();

  @override
  Future<L> load(Locale locale) {
    return SynchronousFuture<L>(lookupL(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'fr'].contains(locale.languageCode);

  @override
  bool shouldReload(_LDelegate old) => false;
}

L lookupL(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return LEn();
    case 'fr':
      return LFr();
  }

  throw FlutterError(
    'L.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
