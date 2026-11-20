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

  /// No description provided for @supportLink.
  ///
  /// In en, this message translates to:
  /// **'Support Rustle3D'**
  String get supportLink;

  /// No description provided for @supportTitle.
  ///
  /// In en, this message translates to:
  /// **'Rustle3D is free and open source'**
  String get supportTitle;

  /// No description provided for @supportText.
  ///
  /// In en, this message translates to:
  /// **'If it is useful to you, a donation helps cover its costs: hosting, test hardware and development time. Everything stays free either way.'**
  String get supportText;

  /// No description provided for @supportDonate.
  ///
  /// In en, this message translates to:
  /// **'Donate'**
  String get supportDonate;

  /// No description provided for @supportLater.
  ///
  /// In en, this message translates to:
  /// **'Not now'**
  String get supportLater;

  /// No description provided for @supportTurnOff.
  ///
  /// In en, this message translates to:
  /// **'Don’t show again'**
  String get supportTurnOff;

  /// No description provided for @updateTitle.
  ///
  /// In en, this message translates to:
  /// **'Rustle3D {version} is available'**
  String updateTitle(String version);

  /// No description provided for @updateText.
  ///
  /// In en, this message translates to:
  /// **'Download it from rustle3d.com and install it over this version: your projects are kept.'**
  String get updateText;

  /// No description provided for @updateDownload.
  ///
  /// In en, this message translates to:
  /// **'Download'**
  String get updateDownload;

  /// No description provided for @updateSkip.
  ///
  /// In en, this message translates to:
  /// **'Skip this version'**
  String get updateSkip;

  /// No description provided for @updateCheck.
  ///
  /// In en, this message translates to:
  /// **'Check for updates'**
  String get updateCheck;

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
  /// **'Import a pattern drawn elsewhere (DXF-AAMA, DXF, SVG, PDF, Seamly2D or Valentina), or a Seamly2D or Valentina measurement file'**
  String get importTip;

  /// No description provided for @importUnsupported.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 kind of drafting step is not supported yet: the pieces using it were left out.} other{{count} kinds of drafting step are not supported yet: the pieces using them were left out.}}'**
  String importUnsupported(int count);

  /// No description provided for @measurementsImported.
  ///
  /// In en, this message translates to:
  /// **'The avatar takes the measurements from the file.'**
  String get measurementsImported;

  /// No description provided for @garmentSource.
  ///
  /// In en, this message translates to:
  /// **'Drafted from {file}'**
  String garmentSource(String file);

  /// No description provided for @garmentRedraft.
  ///
  /// In en, this message translates to:
  /// **'Draft again for the avatar'**
  String get garmentRedraft;

  /// No description provided for @garmentRedraftTip.
  ///
  /// In en, this message translates to:
  /// **'Run the pattern again with the avatar\'s measurements; your changes to the pieces are kept'**
  String get garmentRedraftTip;

  /// No description provided for @busyRedraft.
  ///
  /// In en, this message translates to:
  /// **'Drafting the pattern again'**
  String get busyRedraft;

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

  /// No description provided for @toolSelect.
  ///
  /// In en, this message translates to:
  /// **'Select'**
  String get toolSelect;

  /// No description provided for @toolSelectHint.
  ///
  /// In en, this message translates to:
  /// **'Click a piece; drag it to move it, its frame\'s corners to scale it, the round handle above it to turn it (⇧: 15° steps, or scale both ways at once); drag a corner or an edge to reshape it'**
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
  /// **'Draw'**
  String get toolDraw;

  /// No description provided for @toolDrawHint.
  ///
  /// In en, this message translates to:
  /// **'Drag across a piece, on the pattern or on the 3D garment: the line goes on the pattern'**
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
  /// **'Click two edges or lines on the pattern; drag along one to sew part of it'**
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
  /// **'Corner picked. ⌘-click another corner to measure the distance between them.'**
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
  /// **'Sew: click an edge or a line, or drag along it to sew part of it · ⌘-click edges that follow each other to sew them as one side · V to stop'**
  String get hintSewFirst;

  /// No description provided for @hintSewSecond.
  ///
  /// In en, this message translates to:
  /// **'Sew: now the side to sew it to: click it, drag along part of it or ⌘-click several edges · Esc starts again'**
  String get hintSewSecond;

  /// No description provided for @hintSewRun.
  ///
  /// In en, this message translates to:
  /// **'Sew: ⌘-click the next edges, then click the last one or press Enter · Esc starts again'**
  String get hintSewRun;

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

  /// No description provided for @motionTip.
  ///
  /// In en, this message translates to:
  /// **'Make the avatar move'**
  String get motionTip;

  /// No description provided for @motionStill.
  ///
  /// In en, this message translates to:
  /// **'Stand still'**
  String get motionStill;

  /// No description provided for @motionWalk.
  ///
  /// In en, this message translates to:
  /// **'Walk'**
  String get motionWalk;

  /// No description provided for @motionIdle.
  ///
  /// In en, this message translates to:
  /// **'Stand at ease'**
  String get motionIdle;

  /// No description provided for @motionTurn.
  ///
  /// In en, this message translates to:
  /// **'Turn'**
  String get motionTurn;

  /// No description provided for @motionRaiseArms.
  ///
  /// In en, this message translates to:
  /// **'Raise arms'**
  String get motionRaiseArms;

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

  /// No description provided for @renamePiece.
  ///
  /// In en, this message translates to:
  /// **'Rename the piece (or double-click its name)'**
  String get renamePiece;

  /// No description provided for @renamePieceOk.
  ///
  /// In en, this message translates to:
  /// **'Rename (Enter)'**
  String get renamePieceOk;

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
  /// **'This edge is sewn or zipped: unpick the seam or remove the zipper first.'**
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

  /// A step of the undo history, by its command name with dots as underscores.
  ///
  /// In en, this message translates to:
  /// **'{step, select, project_rename{Rename project} project_set_display_unit{Change units} avatar_set{Change body} avatar_remove{Remove body} panel_create_rect{Add rectangle} panel_create_polygon{Add polygon} panel_create_path{Add curved piece} panel_rename{Rename piece} panel_translate{Move piece} panel_rotate{Rotate piece} panel_scale{Scale piece} panel_set_grain{Set grainline} panel_set_seam_allowance{Set seam allowance} panel_mirror{Mirror piece} panel_unlink{Unlink copy} panel_place{Place piece} panel_delete{Delete piece} panel_cut{Cut piece} panel_paste{Paste} point_move{Move point} point_set_distance{Set distance} point_remove{Remove point} edge_set_curve{Bend edge} edge_split{Add point} edge_move{Move edge} notch_add{Add notch} notch_move{Move notch} notch_remove{Remove notch} internal_line_add{Draw line} internal_line_remove{Remove line} seam_add{Sew} seam_remove{Unsew} seam_flip{Flip seam} seam_set_gathered{Gather seam} edge_lengthen{Lengthen} edge_set_length{Set length} seam_take_in{Take in} seam_match_length{Match lengths} pin_add{Pin} pin_remove{Unpin} garment_set_fabric{Change fabric} garment_arrange{Arrange pieces} garment_set_color{Change colour} garment_add{Add garment} garment_rename{Rename garment} garment_remove{Remove garment} garment_set_layer{Change layer} panel_set_garment{Move to garment} grading_set_sizes{Set sizes} grading_use_chart{Use size chart} grading_clear{Remove sizes} grading_show_size{Show size} point_set_grade{Grade point} template_apply{Add template} pattern_import{Import pattern} batch{Several changes} constraint_add{Add constraint} constraint_remove{Remove constraint} constraint_set_value{Change dimension} point_set_smooth{Smooth point} panel_unfold{Unfold piece} guide_add{Add construction line} guide_move{Move construction line} guide_remove{Remove construction line} panel_create_ellipse{Add ellipse} panel_create_sector{Add circle skirt} edge_set_arc{Make arc} constraint_set_formula{Set formula} variable_set{Set variable} variable_remove{Remove variable} seam_set_ease{Set ease} dart_add{Add dart} pleat_add{Add pleat} panel_add_hole{Cut hole} panel_remove_hole{Fill hole} panel_boolean{Join or cut pieces} other{{step}}}'**
  String stepName(String step);

  /// No description provided for @undoStep.
  ///
  /// In en, this message translates to:
  /// **'Undo {name}  ⌘Z'**
  String undoStep(String name);

  /// No description provided for @redoStep.
  ///
  /// In en, this message translates to:
  /// **'Redo {name}  ⇧⌘Z'**
  String redoStep(String name);

  /// No description provided for @nothingToUndo.
  ///
  /// In en, this message translates to:
  /// **'Nothing to undo'**
  String get nothingToUndo;

  /// No description provided for @nothingToRedo.
  ///
  /// In en, this message translates to:
  /// **'Nothing to redo'**
  String get nothingToRedo;

  /// No description provided for @historyTip.
  ///
  /// In en, this message translates to:
  /// **'History: go back or forward to any step'**
  String get historyTip;

  /// No description provided for @historyTitle.
  ///
  /// In en, this message translates to:
  /// **'History'**
  String get historyTitle;

  /// No description provided for @historyStart.
  ///
  /// In en, this message translates to:
  /// **'Opened'**
  String get historyStart;

  /// No description provided for @historyEmpty.
  ///
  /// In en, this message translates to:
  /// **'No changes yet.'**
  String get historyEmpty;

  /// No description provided for @copied.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 piece copied} other{{count} pieces copied}}, with the seams between them. ⌘V pastes them, here or in another project.'**
  String copied(int count);

  /// No description provided for @pasted.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 piece pasted} other{{count} pieces pasted}}.'**
  String pasted(int count);

  /// No description provided for @nothingToPaste.
  ///
  /// In en, this message translates to:
  /// **'Nothing to paste: copy pieces first (⌘C).'**
  String get nothingToPaste;

  /// No description provided for @selectToCopy.
  ///
  /// In en, this message translates to:
  /// **'Select pieces to copy.'**
  String get selectToCopy;

  /// No description provided for @errCancelled.
  ///
  /// In en, this message translates to:
  /// **'Stopped: nothing was changed.'**
  String get errCancelled;

  /// No description provided for @busyStop.
  ///
  /// In en, this message translates to:
  /// **'Stop'**
  String get busyStop;

  /// No description provided for @busyStep.
  ///
  /// In en, this message translates to:
  /// **'Working…'**
  String get busyStep;

  /// No description provided for @busySizes.
  ///
  /// In en, this message translates to:
  /// **'Drafting size {size} ({done} of {total})'**
  String busySizes(String size, int done, int total);

  /// No description provided for @busySizesStart.
  ///
  /// In en, this message translates to:
  /// **'Drafting every size ({total})'**
  String busySizesStart(int total);

  /// No description provided for @busyBodyFit.
  ///
  /// In en, this message translates to:
  /// **'Fitting the body to the measurements…'**
  String get busyBodyFit;

  /// No description provided for @busyDrape.
  ///
  /// In en, this message translates to:
  /// **'Placing the pieces around the body…'**
  String get busyDrape;

  /// No description provided for @busyExport.
  ///
  /// In en, this message translates to:
  /// **'Exporting…'**
  String get busyExport;

  /// No description provided for @busyImport.
  ///
  /// In en, this message translates to:
  /// **'Reading the pattern…'**
  String get busyImport;

  /// No description provided for @busyOpening.
  ///
  /// In en, this message translates to:
  /// **'Opening the project'**
  String get busyOpening;

  /// No description provided for @busyTemplate.
  ///
  /// In en, this message translates to:
  /// **'Adding the template'**
  String get busyTemplate;

  /// No description provided for @busySizesTitle.
  ///
  /// In en, this message translates to:
  /// **'Setting the sizes'**
  String get busySizesTitle;

  /// No description provided for @busyBodyTitle.
  ///
  /// In en, this message translates to:
  /// **'Changing the body'**
  String get busyBodyTitle;

  /// No description provided for @busyDrapeTitle.
  ///
  /// In en, this message translates to:
  /// **'Starting the drape'**
  String get busyDrapeTitle;

  /// No description provided for @bodyFitting.
  ///
  /// In en, this message translates to:
  /// **'Fitting the body…'**
  String get bodyFitting;

  /// No description provided for @toolShapes.
  ///
  /// In en, this message translates to:
  /// **'Shapes'**
  String get toolShapes;

  /// No description provided for @toolEllipse.
  ///
  /// In en, this message translates to:
  /// **'Ellipse'**
  String get toolEllipse;

  /// No description provided for @toolEllipseHint.
  ///
  /// In en, this message translates to:
  /// **'A circle or an ellipse of the size you type'**
  String get toolEllipseHint;

  /// No description provided for @toolSector.
  ///
  /// In en, this message translates to:
  /// **'Circle skirt'**
  String get toolSector;

  /// No description provided for @toolSectorHint.
  ///
  /// In en, this message translates to:
  /// **'Ring pieces from the waist and the skirt\'s length'**
  String get toolSectorHint;

  /// No description provided for @toolArc.
  ///
  /// In en, this message translates to:
  /// **'Arc'**
  String get toolArc;

  /// No description provided for @toolArcHint.
  ///
  /// In en, this message translates to:
  /// **'Click an edge to make it a circular arc'**
  String get toolArcHint;

  /// No description provided for @toolGuide.
  ///
  /// In en, this message translates to:
  /// **'Construction line'**
  String get toolGuide;

  /// No description provided for @toolGuideHint.
  ///
  /// In en, this message translates to:
  /// **'Click two points: a line to draft against, never cut or sewn'**
  String get toolGuideHint;

  /// No description provided for @hintGuide.
  ///
  /// In en, this message translates to:
  /// **'Construction line: click two points · Esc cancels · V to stop'**
  String get hintGuide;

  /// No description provided for @toolDimension.
  ///
  /// In en, this message translates to:
  /// **'Dimension'**
  String get toolDimension;

  /// No description provided for @toolDimensionHint.
  ///
  /// In en, this message translates to:
  /// **'Click an edge, two corners or two edges, then where the dimension goes; type its value or a formula'**
  String get toolDimensionHint;

  /// No description provided for @hintDimension.
  ///
  /// In en, this message translates to:
  /// **'Dimension: click an edge (its length), a corner then another (the distance) or two edges (the angle) · V to stop'**
  String get hintDimension;

  /// No description provided for @hintDimensionPlace.
  ///
  /// In en, this message translates to:
  /// **'Dimension: click where it goes (between two corners, pull it above or beside them for a width or a height), then type its value · Esc cancels'**
  String get hintDimensionPlace;

  /// No description provided for @hintArc.
  ///
  /// In en, this message translates to:
  /// **'Arc: click an edge · V to stop'**
  String get hintArc;

  /// No description provided for @toastGuideAdded.
  ///
  /// In en, this message translates to:
  /// **'Construction line added.'**
  String get toastGuideAdded;

  /// No description provided for @symmetricHalf.
  ///
  /// In en, this message translates to:
  /// **'Half piece'**
  String get symmetricHalf;

  /// No description provided for @symmetricHalfTip.
  ///
  /// In en, this message translates to:
  /// **'Draw half of a symmetric piece: the closing edge is the axis, the other half follows'**
  String get symmetricHalfTip;

  /// No description provided for @width.
  ///
  /// In en, this message translates to:
  /// **'Width'**
  String get width;

  /// No description provided for @height.
  ///
  /// In en, this message translates to:
  /// **'Height'**
  String get height;

  /// No description provided for @sectorWaist.
  ///
  /// In en, this message translates to:
  /// **'Waist'**
  String get sectorWaist;

  /// No description provided for @sectorLength.
  ///
  /// In en, this message translates to:
  /// **'Skirt length'**
  String get sectorLength;

  /// No description provided for @sectorFull.
  ///
  /// In en, this message translates to:
  /// **'Full'**
  String get sectorFull;

  /// No description provided for @sectorHalf.
  ///
  /// In en, this message translates to:
  /// **'Half'**
  String get sectorHalf;

  /// No description provided for @sectorQuarter.
  ///
  /// In en, this message translates to:
  /// **'Quarter'**
  String get sectorQuarter;

  /// No description provided for @sectorHint.
  ///
  /// In en, this message translates to:
  /// **'A full circle skirt is two half pieces, to sew at the sides.'**
  String get sectorHint;

  /// No description provided for @arcTitle.
  ///
  /// In en, this message translates to:
  /// **'Arc'**
  String get arcTitle;

  /// No description provided for @arcBulge.
  ///
  /// In en, this message translates to:
  /// **'Bulge'**
  String get arcBulge;

  /// No description provided for @arcHint.
  ///
  /// In en, this message translates to:
  /// **'How far the middle of the arc is from the straight line; negative curves inward.'**
  String get arcHint;

  /// No description provided for @constraints.
  ///
  /// In en, this message translates to:
  /// **'Constraints'**
  String get constraints;

  /// No description provided for @constraintsEmpty.
  ///
  /// In en, this message translates to:
  /// **'Pick a corner or an edge on the board (⌘-click a second one) to constrain it.'**
  String get constraintsEmpty;

  /// No description provided for @constraintsFreedom.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{Fully constrained} =1{1 degree of freedom left} other{{count} degrees of freedom left}}'**
  String constraintsFreedom(int count);

  /// No description provided for @constraintsRedundant.
  ///
  /// In en, this message translates to:
  /// **'Some constraints repeat others.'**
  String get constraintsRedundant;

  /// No description provided for @constraintBroken.
  ///
  /// In en, this message translates to:
  /// **'Does not hold'**
  String get constraintBroken;

  /// A constraint or a dimension on the board, by its kind.
  ///
  /// In en, this message translates to:
  /// **'{kind, select, fixed{Fixed} coincident{Joined} horizontal{Horizontal} vertical{Vertical} parallel{Parallel} perpendicular{Square} tangent{Smooth} equal_length{Same length} on_edge{On the edge} distance{Distance} width{Width} height{Height} length{Length} angle{Angle} other{{kind}}}'**
  String constraintKind(String kind);

  /// No description provided for @constrain.
  ///
  /// In en, this message translates to:
  /// **'Constrain'**
  String get constrain;

  /// No description provided for @removeConstraint.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get removeConstraint;

  /// No description provided for @pointSmooth.
  ///
  /// In en, this message translates to:
  /// **'Smooth'**
  String get pointSmooth;

  /// No description provided for @pointCorner.
  ///
  /// In en, this message translates to:
  /// **'Corner'**
  String get pointCorner;

  /// No description provided for @pointRemove.
  ///
  /// In en, this message translates to:
  /// **'Remove point'**
  String get pointRemove;

  /// No description provided for @symmetricPiece.
  ///
  /// In en, this message translates to:
  /// **'Symmetric piece'**
  String get symmetricPiece;

  /// No description provided for @unlinkHalves.
  ///
  /// In en, this message translates to:
  /// **'Separate halves'**
  String get unlinkHalves;

  /// No description provided for @unfold.
  ///
  /// In en, this message translates to:
  /// **'Unfold here'**
  String get unfold;

  /// No description provided for @makeArc.
  ///
  /// In en, this message translates to:
  /// **'Arc'**
  String get makeArc;

  /// What a point snapped to on the board.
  ///
  /// In en, this message translates to:
  /// **'{kind, select, point{Point} midpoint{Middle} edge{On edge} guide{On line} axis{On axis} horizontal{Level} vertical{Aligned} other{{kind}}}'**
  String snapKind(String kind);

  /// No description provided for @errDerived.
  ///
  /// In en, this message translates to:
  /// **'This part follows the drawn half or the original piece: edit that one.'**
  String get errDerived;

  /// No description provided for @errUnsolvable.
  ///
  /// In en, this message translates to:
  /// **'These constraints cannot all hold: nothing was changed.'**
  String get errUnsolvable;

  /// No description provided for @errSymmetric.
  ///
  /// In en, this message translates to:
  /// **'A symmetric piece: separate its halves first.'**
  String get errSymmetric;

  /// No description provided for @errNotStraight.
  ///
  /// In en, this message translates to:
  /// **'This needs a straight edge.'**
  String get errNotStraight;

  /// No description provided for @errDuplicate.
  ///
  /// In en, this message translates to:
  /// **'Already constrained this way.'**
  String get errDuplicate;

  /// No description provided for @formulaTip.
  ///
  /// In en, this message translates to:
  /// **'Formula: compute this value from the body\'s measurements'**
  String get formulaTip;

  /// No description provided for @formulaTitle.
  ///
  /// In en, this message translates to:
  /// **'Formula'**
  String get formulaTitle;

  /// No description provided for @formulaHelp.
  ///
  /// In en, this message translates to:
  /// **'Measurements: height, bust, waist, hips. Units: mm, cm, in. + − × ÷, min, max. Your variables too. Empty: the value stays as it is.'**
  String get formulaHelp;

  /// No description provided for @variables.
  ///
  /// In en, this message translates to:
  /// **'Variables'**
  String get variables;

  /// No description provided for @variablesHint.
  ///
  /// In en, this message translates to:
  /// **'Names for formulas, like ease = 2 cm. Pieces with formulas redraw themselves for a new body and in every size.'**
  String get variablesHint;

  /// No description provided for @variable.
  ///
  /// In en, this message translates to:
  /// **'Variable'**
  String get variable;

  /// No description provided for @variableName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get variableName;

  /// No description provided for @variableAdd.
  ///
  /// In en, this message translates to:
  /// **'Add a variable'**
  String get variableAdd;

  /// No description provided for @errGradedByFormula.
  ///
  /// In en, this message translates to:
  /// **'This point is graded by formulas: change them instead.'**
  String get errGradedByFormula;

  /// No description provided for @errFormulaSyntax.
  ///
  /// In en, this message translates to:
  /// **'The formula cannot be read at character {at}.'**
  String errFormulaSyntax(String at);

  /// No description provided for @errFormulaUnknown.
  ///
  /// In en, this message translates to:
  /// **'“{name}” is neither a measurement nor a variable.'**
  String errFormulaUnknown(String name);

  /// No description provided for @errFormulaNoBody.
  ///
  /// In en, this message translates to:
  /// **'“{name}” needs the body’s measurements: set them first.'**
  String errFormulaNoBody(String name);

  /// No description provided for @errFormulaCycle.
  ///
  /// In en, this message translates to:
  /// **'“{name}” is computed from itself.'**
  String errFormulaCycle(String name);

  /// No description provided for @errFormulaUnit.
  ///
  /// In en, this message translates to:
  /// **'The units do not match (a length needs a unit, like 10 mm).'**
  String get errFormulaUnit;

  /// No description provided for @errFormulaDomain.
  ///
  /// In en, this message translates to:
  /// **'The formula has no value (a division by zero?).'**
  String get errFormulaDomain;

  /// No description provided for @errFormulaRange.
  ///
  /// In en, this message translates to:
  /// **'The formula gives {value}: too small for this dimension.'**
  String errFormulaRange(String value);

  /// No description provided for @errFormulaBadName.
  ///
  /// In en, this message translates to:
  /// **'“{name}” cannot name a variable: a letter, then letters, digits or _.'**
  String errFormulaBadName(String name);

  /// No description provided for @toolDetails.
  ///
  /// In en, this message translates to:
  /// **'Darts and pleats'**
  String get toolDetails;

  /// No description provided for @toolDart.
  ///
  /// In en, this message translates to:
  /// **'Dart'**
  String get toolDart;

  /// No description provided for @toolDartHint.
  ///
  /// In en, this message translates to:
  /// **'Click an edge where the dart goes: drawn, folded and sewn'**
  String get toolDartHint;

  /// No description provided for @toolPleat.
  ///
  /// In en, this message translates to:
  /// **'Pleat'**
  String get toolPleat;

  /// No description provided for @toolPleatHint.
  ///
  /// In en, this message translates to:
  /// **'Click an edge where the pleat starts: folded and sewn'**
  String get toolPleatHint;

  /// No description provided for @toolHole.
  ///
  /// In en, this message translates to:
  /// **'Hole'**
  String get toolHole;

  /// No description provided for @toolHoleHint.
  ///
  /// In en, this message translates to:
  /// **'Select a piece, then click the corners of a hole inside it'**
  String get toolHoleHint;

  /// No description provided for @hintDart.
  ///
  /// In en, this message translates to:
  /// **'Dart: click an edge where it goes · V to stop'**
  String get hintDart;

  /// No description provided for @hintPleat.
  ///
  /// In en, this message translates to:
  /// **'Pleat: click an edge where it starts · V to stop'**
  String get hintPleat;

  /// No description provided for @hintHole.
  ///
  /// In en, this message translates to:
  /// **'Hole: click its corners inside the selected piece, Enter to cut it · Esc cancels'**
  String get hintHole;

  /// No description provided for @toastHoleAdded.
  ///
  /// In en, this message translates to:
  /// **'Hole cut.'**
  String get toastHoleAdded;

  /// No description provided for @holeSelectFirst.
  ///
  /// In en, this message translates to:
  /// **'Select the piece to cut the hole in first.'**
  String get holeSelectFirst;

  /// No description provided for @dartWidth.
  ///
  /// In en, this message translates to:
  /// **'Width at the edge'**
  String get dartWidth;

  /// No description provided for @dartLength.
  ///
  /// In en, this message translates to:
  /// **'Length'**
  String get dartLength;

  /// No description provided for @pleatDepth.
  ///
  /// In en, this message translates to:
  /// **'Depth'**
  String get pleatDepth;

  /// No description provided for @pleatStitched.
  ///
  /// In en, this message translates to:
  /// **'Stitched down'**
  String get pleatStitched;

  /// No description provided for @pleatKnife.
  ///
  /// In en, this message translates to:
  /// **'Knife'**
  String get pleatKnife;

  /// No description provided for @pleatBox.
  ///
  /// In en, this message translates to:
  /// **'Box'**
  String get pleatBox;

  /// No description provided for @combine.
  ///
  /// In en, this message translates to:
  /// **'Join or cut'**
  String get combine;

  /// No description provided for @combineWith.
  ///
  /// In en, this message translates to:
  /// **'With the piece'**
  String get combineWith;

  /// No description provided for @join.
  ///
  /// In en, this message translates to:
  /// **'Join'**
  String get join;

  /// No description provided for @cutOut.
  ///
  /// In en, this message translates to:
  /// **'Cut out'**
  String get cutOut;

  /// No description provided for @ease.
  ///
  /// In en, this message translates to:
  /// **'Ease'**
  String get ease;

  /// No description provided for @errOverlaps.
  ///
  /// In en, this message translates to:
  /// **'This stretch is already sewn.'**
  String get errOverlaps;

  /// No description provided for @errNotWholeEdges.
  ///
  /// In en, this message translates to:
  /// **'This works on seams between two whole edges.'**
  String get errNotWholeEdges;

  /// No description provided for @errApart.
  ///
  /// In en, this message translates to:
  /// **'The result would be several pieces, or nothing.'**
  String get errApart;

  /// No description provided for @errHasHoles.
  ///
  /// In en, this message translates to:
  /// **'This piece has holes: fill them first.'**
  String get errHasHoles;

  /// No description provided for @rotatePiece.
  ///
  /// In en, this message translates to:
  /// **'Rotate'**
  String get rotatePiece;

  /// No description provided for @rotatePieceHint.
  ///
  /// In en, this message translates to:
  /// **'Counter-clockwise; a negative angle turns the other way.'**
  String get rotatePieceHint;

  /// No description provided for @rotateAngle.
  ///
  /// In en, this message translates to:
  /// **'Angle'**
  String get rotateAngle;

  /// No description provided for @scalePiece.
  ///
  /// In en, this message translates to:
  /// **'Scale'**
  String get scalePiece;

  /// No description provided for @scaleAcross.
  ///
  /// In en, this message translates to:
  /// **'Across'**
  String get scaleAcross;

  /// No description provided for @scaleUpDown.
  ///
  /// In en, this message translates to:
  /// **'Up and down'**
  String get scaleUpDown;

  /// No description provided for @errConstrained.
  ///
  /// In en, this message translates to:
  /// **'This piece has dimensions or constraints that would no longer hold: remove them first.'**
  String get errConstrained;

  /// No description provided for @errNotJoined.
  ///
  /// In en, this message translates to:
  /// **'These edges do not follow each other on one piece.'**
  String get errNotJoined;

  /// No description provided for @close.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get close;

  /// No description provided for @marker.
  ///
  /// In en, this message translates to:
  /// **'Marker'**
  String get marker;

  /// No description provided for @exportMarker.
  ///
  /// In en, this message translates to:
  /// **'Marker: the pieces laid out on the fabric…'**
  String get exportMarker;

  /// No description provided for @markerHint.
  ///
  /// In en, this message translates to:
  /// **'The pieces, with their seam allowances, laid out on the fabric along its grain. Pick the sizes to cut and the fabric\'s width: you get the length to buy and its cost.'**
  String get markerHint;

  /// No description provided for @markerGarment.
  ///
  /// In en, this message translates to:
  /// **'Garment'**
  String get markerGarment;

  /// No description provided for @markerSizes.
  ///
  /// In en, this message translates to:
  /// **'Sizes to cut'**
  String get markerSizes;

  /// No description provided for @markerWidth.
  ///
  /// In en, this message translates to:
  /// **'Fabric width'**
  String get markerWidth;

  /// No description provided for @markerLayOut.
  ///
  /// In en, this message translates to:
  /// **'Lay out'**
  String get markerLayOut;

  /// No description provided for @markerLength.
  ///
  /// In en, this message translates to:
  /// **'Length'**
  String get markerLength;

  /// No description provided for @markerEfficiency.
  ///
  /// In en, this message translates to:
  /// **'Fabric used'**
  String get markerEfficiency;

  /// No description provided for @markerYardage.
  ///
  /// In en, this message translates to:
  /// **'To buy'**
  String get markerYardage;

  /// No description provided for @markerCost.
  ///
  /// In en, this message translates to:
  /// **'Cost'**
  String get markerCost;

  /// No description provided for @markerNoPrice.
  ///
  /// In en, this message translates to:
  /// **'Give the fabric a price per metre (inspector, with nothing selected, under the garment) to see the cost.'**
  String get markerNoPrice;

  /// No description provided for @markerSaveSvg.
  ///
  /// In en, this message translates to:
  /// **'Save SVG'**
  String get markerSaveSvg;

  /// No description provided for @markerSavePdf.
  ///
  /// In en, this message translates to:
  /// **'Save PDF'**
  String get markerSavePdf;

  /// No description provided for @tpDecimal.
  ///
  /// In en, this message translates to:
  /// **'.'**
  String get tpDecimal;

  /// No description provided for @tpSizes.
  ///
  /// In en, this message translates to:
  /// **'Sizes'**
  String get tpSizes;

  /// No description provided for @tpSample.
  ///
  /// In en, this message translates to:
  /// **'sample'**
  String get tpSample;

  /// No description provided for @tpViews.
  ///
  /// In en, this message translates to:
  /// **'Views'**
  String get tpViews;

  /// No description provided for @tpFront.
  ///
  /// In en, this message translates to:
  /// **'Front'**
  String get tpFront;

  /// No description provided for @tpBack.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get tpBack;

  /// No description provided for @tpColourways.
  ///
  /// In en, this message translates to:
  /// **'Colourways'**
  String get tpColourways;

  /// No description provided for @tpMaterials.
  ///
  /// In en, this message translates to:
  /// **'Bill of materials'**
  String get tpMaterials;

  /// No description provided for @tpFabrics.
  ///
  /// In en, this message translates to:
  /// **'Fabrics'**
  String get tpFabrics;

  /// No description provided for @tpGarment.
  ///
  /// In en, this message translates to:
  /// **'Garment'**
  String get tpGarment;

  /// No description provided for @tpFabric.
  ///
  /// In en, this message translates to:
  /// **'Fabric'**
  String get tpFabric;

  /// No description provided for @tpColour.
  ///
  /// In en, this message translates to:
  /// **'Colour'**
  String get tpColour;

  /// No description provided for @tpWeight.
  ///
  /// In en, this message translates to:
  /// **'Weight (g/m²)'**
  String get tpWeight;

  /// No description provided for @tpWidth.
  ///
  /// In en, this message translates to:
  /// **'Width (cm)'**
  String get tpWidth;

  /// No description provided for @tpPrice.
  ///
  /// In en, this message translates to:
  /// **'Price per metre'**
  String get tpPrice;

  /// No description provided for @tpConsumption.
  ///
  /// In en, this message translates to:
  /// **'Fabric per garment (m)'**
  String get tpConsumption;

  /// No description provided for @tpCost.
  ///
  /// In en, this message translates to:
  /// **'Cost per garment'**
  String get tpCost;

  /// No description provided for @tpConsumptionBySize.
  ///
  /// In en, this message translates to:
  /// **'Fabric per garment by size (m)'**
  String get tpConsumptionBySize;

  /// No description provided for @tpTrims.
  ///
  /// In en, this message translates to:
  /// **'Trims'**
  String get tpTrims;

  /// No description provided for @tpNoTrims.
  ///
  /// In en, this message translates to:
  /// **'No trims.'**
  String get tpNoTrims;

  /// No description provided for @tpMeasurements.
  ///
  /// In en, this message translates to:
  /// **'Measurements'**
  String get tpMeasurements;

  /// No description provided for @tpSizeChart.
  ///
  /// In en, this message translates to:
  /// **'Size chart (body, cm)'**
  String get tpSizeChart;

  /// No description provided for @tpSize.
  ///
  /// In en, this message translates to:
  /// **'Size'**
  String get tpSize;

  /// No description provided for @tpHeight.
  ///
  /// In en, this message translates to:
  /// **'Height'**
  String get tpHeight;

  /// No description provided for @tpBust.
  ///
  /// In en, this message translates to:
  /// **'Bust'**
  String get tpBust;

  /// No description provided for @tpWaist.
  ///
  /// In en, this message translates to:
  /// **'Waist'**
  String get tpWaist;

  /// No description provided for @tpHips.
  ///
  /// In en, this message translates to:
  /// **'Hips'**
  String get tpHips;

  /// No description provided for @tpPieces.
  ///
  /// In en, this message translates to:
  /// **'Pieces'**
  String get tpPieces;

  /// No description provided for @tpPiece.
  ///
  /// In en, this message translates to:
  /// **'Piece'**
  String get tpPiece;

  /// No description provided for @tpLength.
  ///
  /// In en, this message translates to:
  /// **'Length (cm)'**
  String get tpLength;

  /// No description provided for @tpPieceWidth.
  ///
  /// In en, this message translates to:
  /// **'Width (cm)'**
  String get tpPieceWidth;

  /// No description provided for @tpPerimeter.
  ///
  /// In en, this message translates to:
  /// **'Sewing line (cm)'**
  String get tpPerimeter;

  /// No description provided for @tpArea.
  ///
  /// In en, this message translates to:
  /// **'Area (cm²)'**
  String get tpArea;

  /// No description provided for @tpCutArea.
  ///
  /// In en, this message translates to:
  /// **'Cut area (cm²)'**
  String get tpCutArea;

  /// No description provided for @tpAllowance.
  ///
  /// In en, this message translates to:
  /// **'Allowance (mm)'**
  String get tpAllowance;

  /// No description provided for @tpMass.
  ///
  /// In en, this message translates to:
  /// **'Mass (g)'**
  String get tpMass;

  /// No description provided for @tpPiecesBySize.
  ///
  /// In en, this message translates to:
  /// **'Pieces by size: length x width (cm)'**
  String get tpPiecesBySize;

  /// No description provided for @tpPattern.
  ///
  /// In en, this message translates to:
  /// **'Pattern'**
  String get tpPattern;

  /// No description provided for @tpSewing.
  ///
  /// In en, this message translates to:
  /// **'Sewing notes'**
  String get tpSewing;

  /// No description provided for @tpSeams.
  ///
  /// In en, this message translates to:
  /// **'Seams'**
  String get tpSeams;

  /// No description provided for @tpSideA.
  ///
  /// In en, this message translates to:
  /// **'Side A'**
  String get tpSideA;

  /// No description provided for @tpSideB.
  ///
  /// In en, this message translates to:
  /// **'Side B'**
  String get tpSideB;

  /// No description provided for @tpLengthA.
  ///
  /// In en, this message translates to:
  /// **'Length A (mm)'**
  String get tpLengthA;

  /// No description provided for @tpLengthB.
  ///
  /// In en, this message translates to:
  /// **'Length B (mm)'**
  String get tpLengthB;

  /// No description provided for @tpEase.
  ///
  /// In en, this message translates to:
  /// **'Ease (mm)'**
  String get tpEase;

  /// No description provided for @tpGathered.
  ///
  /// In en, this message translates to:
  /// **'Gathered'**
  String get tpGathered;

  /// No description provided for @tpYes.
  ///
  /// In en, this message translates to:
  /// **'yes'**
  String get tpYes;

  /// No description provided for @tpNoSeams.
  ///
  /// In en, this message translates to:
  /// **'No seams yet.'**
  String get tpNoSeams;

  /// No description provided for @tpNotes.
  ///
  /// In en, this message translates to:
  /// **'Notes'**
  String get tpNotes;

  /// No description provided for @exportTechpack.
  ///
  /// In en, this message translates to:
  /// **'Tech pack (PDF)…'**
  String get exportTechpack;

  /// No description provided for @exportProjector.
  ///
  /// In en, this message translates to:
  /// **'Projector: the pattern at 1:1…'**
  String get exportProjector;

  /// No description provided for @techpackLetter.
  ///
  /// In en, this message translates to:
  /// **'Letter pages'**
  String get techpackLetter;

  /// No description provided for @garmentNotes.
  ///
  /// In en, this message translates to:
  /// **'Notes'**
  String get garmentNotes;

  /// No description provided for @garmentNotesHint.
  ///
  /// In en, this message translates to:
  /// **'Sewing and finishing notes, printed in the tech pack'**
  String get garmentNotesHint;

  /// No description provided for @projectorHelp.
  ///
  /// In en, this message translates to:
  /// **'Drag or use the arrow keys to bring the pattern under the projector (Shift: 10 cm). Home: back to the middle. M: mirror · R: turn · I: contrast · C: calibrate · Esc: close.'**
  String get projectorHelp;

  /// No description provided for @projectorCalibrate.
  ///
  /// In en, this message translates to:
  /// **'Calibrate'**
  String get projectorCalibrate;

  /// No description provided for @projectorMirror.
  ///
  /// In en, this message translates to:
  /// **'Mirror'**
  String get projectorMirror;

  /// No description provided for @projectorTurn.
  ///
  /// In en, this message translates to:
  /// **'Turn'**
  String get projectorTurn;

  /// No description provided for @projectorContrast.
  ///
  /// In en, this message translates to:
  /// **'Contrast'**
  String get projectorContrast;

  /// No description provided for @projectorCalibrateHint.
  ///
  /// In en, this message translates to:
  /// **'The square below should be 20 cm a side on the table. Measure its sides with a ruler and type them in.'**
  String get projectorCalibrateHint;

  /// No description provided for @projectorMeasuredWidth.
  ///
  /// In en, this message translates to:
  /// **'Width measured'**
  String get projectorMeasuredWidth;

  /// No description provided for @projectorMeasuredHeight.
  ///
  /// In en, this message translates to:
  /// **'Height measured'**
  String get projectorMeasuredHeight;

  /// No description provided for @projectorApply.
  ///
  /// In en, this message translates to:
  /// **'Apply'**
  String get projectorApply;

  /// No description provided for @projectorReset.
  ///
  /// In en, this message translates to:
  /// **'Reset'**
  String get projectorReset;

  /// No description provided for @projectorNothing.
  ///
  /// In en, this message translates to:
  /// **'No piece to project.'**
  String get projectorNothing;

  /// No description provided for @fabricBought.
  ///
  /// In en, this message translates to:
  /// **'As bought'**
  String get fabricBought;

  /// No description provided for @fabricWeight.
  ///
  /// In en, this message translates to:
  /// **'Weight'**
  String get fabricWeight;

  /// No description provided for @fabricWidth.
  ///
  /// In en, this message translates to:
  /// **'Width'**
  String get fabricWidth;

  /// No description provided for @fabricPrice.
  ///
  /// In en, this message translates to:
  /// **'Price / m'**
  String get fabricPrice;

  /// No description provided for @cutArea.
  ///
  /// In en, this message translates to:
  /// **'Cut area'**
  String get cutArea;

  /// No description provided for @pieceMass.
  ///
  /// In en, this message translates to:
  /// **'Mass'**
  String get pieceMass;

  /// No description provided for @errMarkerTooNarrow.
  ///
  /// In en, this message translates to:
  /// **'A piece is wider than the fabric: choose a wider fabric.'**
  String get errMarkerTooNarrow;

  /// No description provided for @garmentTotals.
  ///
  /// In en, this message translates to:
  /// **'{pieces} pieces · {area} m² of fabric cut · {mass} g'**
  String garmentTotals(int pieces, String area, String mass);

  /// No description provided for @fitTip.
  ///
  /// In en, this message translates to:
  /// **'Show where the garment is tight'**
  String get fitTip;

  /// No description provided for @fitOff.
  ///
  /// In en, this message translates to:
  /// **'Fabric colours'**
  String get fitOff;

  /// No description provided for @fitStrain.
  ///
  /// In en, this message translates to:
  /// **'Strain'**
  String get fitStrain;

  /// No description provided for @fitGap.
  ///
  /// In en, this message translates to:
  /// **'Gap to the body'**
  String get fitGap;

  /// No description provided for @fitStrainTitle.
  ///
  /// In en, this message translates to:
  /// **'Strain: stretch beyond the pattern'**
  String get fitStrainTitle;

  /// No description provided for @fitGapTitle.
  ///
  /// In en, this message translates to:
  /// **'Gap to the body'**
  String get fitGapTitle;

  /// No description provided for @fitLoose.
  ///
  /// In en, this message translates to:
  /// **'loose'**
  String get fitLoose;

  /// No description provided for @fitTight.
  ///
  /// In en, this message translates to:
  /// **'tight'**
  String get fitTight;

  /// No description provided for @fitTightest.
  ///
  /// In en, this message translates to:
  /// **'Tightest: {value} on {piece}'**
  String fitTightest(String value, String piece);

  /// No description provided for @toolTape.
  ///
  /// In en, this message translates to:
  /// **'Tape measure'**
  String get toolTape;

  /// No description provided for @toolTapeHint.
  ///
  /// In en, this message translates to:
  /// **'Click on the 3D garment: its circumference there, the body\'s and the ease; or a length between two points'**
  String get toolTapeHint;

  /// No description provided for @hintTape.
  ///
  /// In en, this message translates to:
  /// **'Tape measure: click on the 3D garment to measure around it there, or two points for a length · drag to turn the view · V to stop'**
  String get hintTape;

  /// No description provided for @tapeHint.
  ///
  /// In en, this message translates to:
  /// **'Click on the draped garment.'**
  String get tapeHint;

  /// No description provided for @tapeClose.
  ///
  /// In en, this message translates to:
  /// **'Put the tape away'**
  String get tapeClose;

  /// No description provided for @tapeHeight.
  ///
  /// In en, this message translates to:
  /// **'Height'**
  String get tapeHeight;

  /// No description provided for @tapeGarment.
  ///
  /// In en, this message translates to:
  /// **'Garment'**
  String get tapeGarment;

  /// No description provided for @tapeBody.
  ///
  /// In en, this message translates to:
  /// **'Body'**
  String get tapeBody;

  /// No description provided for @tapeEase.
  ///
  /// In en, this message translates to:
  /// **'Ease'**
  String get tapeEase;

  /// No description provided for @tapeGap.
  ///
  /// In en, this message translates to:
  /// **'Gap here'**
  String get tapeGap;

  /// No description provided for @tapeOpen.
  ///
  /// In en, this message translates to:
  /// **'The garment does not go all round here: no ease.'**
  String get tapeOpen;

  /// No description provided for @tapeRound.
  ///
  /// In en, this message translates to:
  /// **'Round'**
  String get tapeRound;

  /// No description provided for @tapeLength.
  ///
  /// In en, this message translates to:
  /// **'Length'**
  String get tapeLength;

  /// No description provided for @tapeFirstPoint.
  ///
  /// In en, this message translates to:
  /// **'Click the first point on the draped garment.'**
  String get tapeFirstPoint;

  /// No description provided for @tapeSecondPoint.
  ///
  /// In en, this message translates to:
  /// **'Click the second point.'**
  String get tapeSecondPoint;

  /// No description provided for @tapeAlong.
  ///
  /// In en, this message translates to:
  /// **'Along the cloth'**
  String get tapeAlong;

  /// No description provided for @tapeTaut.
  ///
  /// In en, this message translates to:
  /// **'Tape pulled taut'**
  String get tapeTaut;

  /// No description provided for @tapeStraight.
  ///
  /// In en, this message translates to:
  /// **'Straight'**
  String get tapeStraight;

  /// No description provided for @tapeNotJoined.
  ///
  /// In en, this message translates to:
  /// **'The cloth does not join these points: straight only.'**
  String get tapeNotJoined;

  /// No description provided for @printTitle.
  ///
  /// In en, this message translates to:
  /// **'Print'**
  String get printTitle;

  /// No description provided for @printHint.
  ///
  /// In en, this message translates to:
  /// **'Lies along each piece\'s grainline, at its real size.'**
  String get printHint;

  /// No description provided for @printNone.
  ///
  /// In en, this message translates to:
  /// **'No print'**
  String get printNone;

  /// No description provided for @printStripes.
  ///
  /// In en, this message translates to:
  /// **'Stripes'**
  String get printStripes;

  /// No description provided for @printGingham.
  ///
  /// In en, this message translates to:
  /// **'Gingham'**
  String get printGingham;

  /// No description provided for @printDots.
  ///
  /// In en, this message translates to:
  /// **'Dots'**
  String get printDots;

  /// No description provided for @printTwill.
  ///
  /// In en, this message translates to:
  /// **'Twill'**
  String get printTwill;

  /// No description provided for @printImage.
  ///
  /// In en, this message translates to:
  /// **'Your image (PNG, JPEG)…'**
  String get printImage;

  /// No description provided for @printImages.
  ///
  /// In en, this message translates to:
  /// **'Images'**
  String get printImages;

  /// No description provided for @printMissing.
  ///
  /// In en, this message translates to:
  /// **'The image cannot be read: the plain colour is shown.'**
  String get printMissing;

  /// No description provided for @printWidth.
  ///
  /// In en, this message translates to:
  /// **'Repeat width'**
  String get printWidth;

  /// No description provided for @printAngle.
  ///
  /// In en, this message translates to:
  /// **'Angle to the grain'**
  String get printAngle;

  /// No description provided for @piecePrint.
  ///
  /// In en, this message translates to:
  /// **'Print on this piece'**
  String get piecePrint;

  /// No description provided for @piecePrintAcross.
  ///
  /// In en, this message translates to:
  /// **'Shift across'**
  String get piecePrintAcross;

  /// No description provided for @piecePrintUp.
  ///
  /// In en, this message translates to:
  /// **'Shift up'**
  String get piecePrintUp;

  /// No description provided for @piecePrintTurn.
  ///
  /// In en, this message translates to:
  /// **'Turn'**
  String get piecePrintTurn;

  /// No description provided for @errTextureFormat.
  ///
  /// In en, this message translates to:
  /// **'This file is not a PNG or JPEG image.'**
  String get errTextureFormat;

  /// No description provided for @errTextureBad.
  ///
  /// In en, this message translates to:
  /// **'This image cannot be read.'**
  String get errTextureBad;

  /// No description provided for @errTextureTooLarge.
  ///
  /// In en, this message translates to:
  /// **'This image is too large (16384 pixels a side at most).'**
  String get errTextureTooLarge;

  /// No description provided for @toolTrims.
  ///
  /// In en, this message translates to:
  /// **'Trims'**
  String get toolTrims;

  /// No description provided for @toolButton.
  ///
  /// In en, this message translates to:
  /// **'Button'**
  String get toolButton;

  /// No description provided for @toolButtonHint.
  ///
  /// In en, this message translates to:
  /// **'Click where the button goes, then where it buttons: the end of the buttonhole, on the piece that laps over'**
  String get toolButtonHint;

  /// No description provided for @toolZipper.
  ///
  /// In en, this message translates to:
  /// **'Zipper'**
  String get toolZipper;

  /// No description provided for @toolZipperHint.
  ///
  /// In en, this message translates to:
  /// **'Click one edge near the zipper\'s bottom, then the edge it zips to'**
  String get toolZipperHint;

  /// No description provided for @toastButtonAdded.
  ///
  /// In en, this message translates to:
  /// **'Button added.'**
  String get toastButtonAdded;

  /// No description provided for @toastZipperAdded.
  ///
  /// In en, this message translates to:
  /// **'Zipper added.'**
  String get toastZipperAdded;

  /// No description provided for @buttonClickPiece.
  ///
  /// In en, this message translates to:
  /// **'Click on a piece.'**
  String get buttonClickPiece;

  /// No description provided for @trimsTitle.
  ///
  /// In en, this message translates to:
  /// **'Buttons and zippers'**
  String get trimsTitle;

  /// No description provided for @trimButton.
  ///
  /// In en, this message translates to:
  /// **'Button · {mm} mm'**
  String trimButton(String mm);

  /// No description provided for @trimButtonhole.
  ///
  /// In en, this message translates to:
  /// **'Buttonhole · {mm} mm button on {piece}'**
  String trimButtonhole(String mm, String piece);

  /// No description provided for @trimButtoned.
  ///
  /// In en, this message translates to:
  /// **'Buttoned'**
  String get trimButtoned;

  /// No description provided for @trimZipper.
  ///
  /// In en, this message translates to:
  /// **'Zipper · {cm} cm'**
  String trimZipper(String cm);

  /// No description provided for @trimOpen.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get trimOpen;

  /// No description provided for @buttonAll.
  ///
  /// In en, this message translates to:
  /// **'Button all'**
  String get buttonAll;

  /// No description provided for @unbuttonAll.
  ///
  /// In en, this message translates to:
  /// **'Unbutton all'**
  String get unbuttonAll;

  /// No description provided for @trimRemove.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get trimRemove;

  /// No description provided for @tpTrim.
  ///
  /// In en, this message translates to:
  /// **'Trim'**
  String get tpTrim;

  /// No description provided for @tpTrimSize.
  ///
  /// In en, this message translates to:
  /// **'Size'**
  String get tpTrimSize;

  /// No description provided for @tpQuantity.
  ///
  /// In en, this message translates to:
  /// **'Quantity'**
  String get tpQuantity;

  /// No description provided for @tpPlacement.
  ///
  /// In en, this message translates to:
  /// **'On'**
  String get tpPlacement;

  /// No description provided for @tpButton.
  ///
  /// In en, this message translates to:
  /// **'Button'**
  String get tpButton;

  /// No description provided for @tpZipper.
  ///
  /// In en, this message translates to:
  /// **'Zipper'**
  String get tpZipper;

  /// No description provided for @errOutsidePiece.
  ///
  /// In en, this message translates to:
  /// **'Put it inside the piece.'**
  String get errOutsidePiece;

  /// No description provided for @bodyMenuMeasurements.
  ///
  /// In en, this message translates to:
  /// **'Measurements…'**
  String get bodyMenuMeasurements;

  /// No description provided for @bodyMenuImport.
  ///
  /// In en, this message translates to:
  /// **'Import body…'**
  String get bodyMenuImport;

  /// No description provided for @bodyMenuCheck.
  ///
  /// In en, this message translates to:
  /// **'Check body…'**
  String get bodyMenuCheck;

  /// No description provided for @bodyMenuPoints.
  ///
  /// In en, this message translates to:
  /// **'Show arrangement points'**
  String get bodyMenuPoints;

  /// No description provided for @bodyMenuBuiltin.
  ///
  /// In en, this message translates to:
  /// **'Use the built-in body'**
  String get bodyMenuBuiltin;

  /// No description provided for @bodyFiles.
  ///
  /// In en, this message translates to:
  /// **'3D bodies (OBJ, glTF, GLB)'**
  String get bodyFiles;

  /// No description provided for @bodyImported.
  ///
  /// In en, this message translates to:
  /// **'Body imported: check it before draping.'**
  String get bodyImported;

  /// No description provided for @bodyBuiltinBack.
  ///
  /// In en, this message translates to:
  /// **'Back to the built-in body.'**
  String get bodyBuiltinBack;

  /// No description provided for @bodyCheckTitle.
  ///
  /// In en, this message translates to:
  /// **'Check body'**
  String get bodyCheckTitle;

  /// No description provided for @bodyCheckSummary.
  ///
  /// In en, this message translates to:
  /// **'{file} · {triangles} triangles · {height} m tall'**
  String bodyCheckSummary(String file, String triangles, String height);

  /// No description provided for @bodyCheckReady.
  ///
  /// In en, this message translates to:
  /// **'It can be draped.'**
  String get bodyCheckReady;

  /// No description provided for @bodyCheckNotReady.
  ///
  /// In en, this message translates to:
  /// **'It cannot be draped yet: see below.'**
  String get bodyCheckNotReady;

  /// No description provided for @bodyCheckAllGood.
  ///
  /// In en, this message translates to:
  /// **'Nothing to fix.'**
  String get bodyCheckAllGood;

  /// No description provided for @bodyReading.
  ///
  /// In en, this message translates to:
  /// **'How the file is read'**
  String get bodyReading;

  /// No description provided for @bodyUnits.
  ///
  /// In en, this message translates to:
  /// **'Units'**
  String get bodyUnits;

  /// No description provided for @unitMetres.
  ///
  /// In en, this message translates to:
  /// **'Metres'**
  String get unitMetres;

  /// No description provided for @unitCentimetres.
  ///
  /// In en, this message translates to:
  /// **'Centimetres'**
  String get unitCentimetres;

  /// No description provided for @unitMillimetres.
  ///
  /// In en, this message translates to:
  /// **'Millimetres'**
  String get unitMillimetres;

  /// No description provided for @unitInches.
  ///
  /// In en, this message translates to:
  /// **'Inches'**
  String get unitInches;

  /// No description provided for @unitOther.
  ///
  /// In en, this message translates to:
  /// **'{metres} m per unit'**
  String unitOther(String metres);

  /// No description provided for @bodyUp.
  ///
  /// In en, this message translates to:
  /// **'Up axis'**
  String get bodyUp;

  /// No description provided for @bodyTurn.
  ///
  /// In en, this message translates to:
  /// **'Turn'**
  String get bodyTurn;

  /// No description provided for @bodyPoints.
  ///
  /// In en, this message translates to:
  /// **'Arrangement points'**
  String get bodyPoints;

  /// No description provided for @bodyPointsHint.
  ///
  /// In en, this message translates to:
  /// **'Inside the body, in cm: across (left +), height, depth (front +). Drag them in the 3D view with Body › Show arrangement points.'**
  String get bodyPointsHint;

  /// No description provided for @bodyGuessAgain.
  ///
  /// In en, this message translates to:
  /// **'Guess again'**
  String get bodyGuessAgain;

  /// No description provided for @bodyHasWarnings.
  ///
  /// In en, this message translates to:
  /// **'The body has things to check.'**
  String get bodyHasWarnings;

  /// No description provided for @motionImportedTip.
  ///
  /// In en, this message translates to:
  /// **'An imported body stands still'**
  String get motionImportedTip;

  /// No description provided for @pointChest.
  ///
  /// In en, this message translates to:
  /// **'Chest'**
  String get pointChest;

  /// No description provided for @pointWaist.
  ///
  /// In en, this message translates to:
  /// **'Waist'**
  String get pointWaist;

  /// No description provided for @pointHips.
  ///
  /// In en, this message translates to:
  /// **'Hips'**
  String get pointHips;

  /// No description provided for @pointCrotch.
  ///
  /// In en, this message translates to:
  /// **'Crotch'**
  String get pointCrotch;

  /// No description provided for @pointLeftShoulder.
  ///
  /// In en, this message translates to:
  /// **'Left shoulder'**
  String get pointLeftShoulder;

  /// No description provided for @pointLeftElbow.
  ///
  /// In en, this message translates to:
  /// **'Left elbow'**
  String get pointLeftElbow;

  /// No description provided for @pointLeftWrist.
  ///
  /// In en, this message translates to:
  /// **'Left wrist'**
  String get pointLeftWrist;

  /// No description provided for @pointRightShoulder.
  ///
  /// In en, this message translates to:
  /// **'Right shoulder'**
  String get pointRightShoulder;

  /// No description provided for @pointRightElbow.
  ///
  /// In en, this message translates to:
  /// **'Right elbow'**
  String get pointRightElbow;

  /// No description provided for @pointRightWrist.
  ///
  /// In en, this message translates to:
  /// **'Right wrist'**
  String get pointRightWrist;

  /// No description provided for @pointLeftThigh.
  ///
  /// In en, this message translates to:
  /// **'Left thigh'**
  String get pointLeftThigh;

  /// No description provided for @pointLeftShin.
  ///
  /// In en, this message translates to:
  /// **'Left shin'**
  String get pointLeftShin;

  /// No description provided for @pointRightThigh.
  ///
  /// In en, this message translates to:
  /// **'Right thigh'**
  String get pointRightThigh;

  /// No description provided for @pointRightShin.
  ///
  /// In en, this message translates to:
  /// **'Right shin'**
  String get pointRightShin;

  /// No description provided for @checkFileUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The body\'s file {file} is missing or was changed: import it again.'**
  String checkFileUnavailable(String file);

  /// No description provided for @checkMeshOpen.
  ///
  /// In en, this message translates to:
  /// **'{holes} holes in the surface ({edges} open edges): closed for the cloth as well as possible.'**
  String checkMeshOpen(String holes, String edges);

  /// No description provided for @checkMeshNonManifold.
  ///
  /// In en, this message translates to:
  /// **'{edges} edges are shared by more than two triangles.'**
  String checkMeshNonManifold(String edges);

  /// No description provided for @checkMeshPieces.
  ///
  /// In en, this message translates to:
  /// **'{pieces} loose parts besides the body.'**
  String checkMeshPieces(String pieces);

  /// No description provided for @checkMeshHeavy.
  ///
  /// In en, this message translates to:
  /// **'{triangles} triangles: the views and the drape will be slower.'**
  String checkMeshHeavy(String triangles);

  /// No description provided for @checkMeshCoarse.
  ///
  /// In en, this message translates to:
  /// **'Only {triangles} triangles: the body will look faceted.'**
  String checkMeshCoarse(String triangles);

  /// No description provided for @checkMeshReoriented.
  ///
  /// In en, this message translates to:
  /// **'{triangles} triangles were turned to face out like their neighbours.'**
  String checkMeshReoriented(String triangles);

  /// No description provided for @checkMeshInsideOut.
  ///
  /// In en, this message translates to:
  /// **'The surface faced inward: it was turned.'**
  String get checkMeshInsideOut;

  /// No description provided for @checkMeshDegenerate.
  ///
  /// In en, this message translates to:
  /// **'{triangles} flat triangles are ignored.'**
  String checkMeshDegenerate(String triangles);

  /// No description provided for @checkScaleHeight.
  ///
  /// In en, this message translates to:
  /// **'{height} m tall: are the units right?'**
  String checkScaleHeight(String height);

  /// No description provided for @checkOrientationUp.
  ///
  /// In en, this message translates to:
  /// **'The body is wider than it is tall: is the up axis right?'**
  String get checkOrientationUp;

  /// No description provided for @checkOrientationFacing.
  ///
  /// In en, this message translates to:
  /// **'The feet point backwards: turn the body 180°.'**
  String get checkOrientationFacing;

  /// No description provided for @checkPointsOutside.
  ///
  /// In en, this message translates to:
  /// **'{point} is outside the body.'**
  String checkPointsOutside(String point);

  /// No description provided for @checkPointsSide.
  ///
  /// In en, this message translates to:
  /// **'{point} is on the wrong side of the body.'**
  String checkPointsSide(String point);

  /// No description provided for @checkMeasurementsClamped.
  ///
  /// In en, this message translates to:
  /// **'A measurement ({measurement}) was out of range and was brought into it.'**
  String checkMeasurementsClamped(String measurement);

  /// No description provided for @checkMotionUnavailable.
  ///
  /// In en, this message translates to:
  /// **'An imported body stands still: motions are off.'**
  String get checkMotionUnavailable;

  /// No description provided for @errModelMalformed.
  ///
  /// In en, this message translates to:
  /// **'This {format} file cannot be read: {detail}'**
  String errModelMalformed(String format, String detail);

  /// No description provided for @errModelExternalBuffers.
  ///
  /// In en, this message translates to:
  /// **'This glTF keeps its data in separate files: save it as a single .glb.'**
  String get errModelExternalBuffers;

  /// No description provided for @errModelCompressed.
  ///
  /// In en, this message translates to:
  /// **'This glTF is compressed ({extension}): save it without compression.'**
  String errModelCompressed(String extension);

  /// No description provided for @errModelEmpty.
  ///
  /// In en, this message translates to:
  /// **'This file holds no triangles.'**
  String get errModelEmpty;

  /// No description provided for @errModelTooLarge.
  ///
  /// In en, this message translates to:
  /// **'This body has {triangles} triangles, more than {limit}: simplify it first.'**
  String errModelTooLarge(String triangles, String limit);

  /// No description provided for @errBodyUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The body\'s file {file} is missing or was changed: import it again.'**
  String errBodyUnavailable(String file);

  /// No description provided for @errImportedBodyStill.
  ///
  /// In en, this message translates to:
  /// **'An imported body cannot move yet.'**
  String get errImportedBodyStill;

  /// No description provided for @errNoImportedBody.
  ///
  /// In en, this message translates to:
  /// **'The avatar has no imported body.'**
  String get errNoImportedBody;

  /// No description provided for @exportGame.
  ///
  /// In en, this message translates to:
  /// **'Garment for a game engine (glTF)…'**
  String get exportGame;

  /// No description provided for @gameTitle.
  ///
  /// In en, this message translates to:
  /// **'Export for a game engine'**
  String get gameTitle;

  /// No description provided for @gameIntro.
  ///
  /// In en, this message translates to:
  /// **'The garment as the 3D view shows it, in one GLB file: lighter levels of detail, its wrinkles baked into normal maps, and moving with the body\'s skeleton.'**
  String get gameIntro;

  /// No description provided for @gameEngine.
  ///
  /// In en, this message translates to:
  /// **'Engine'**
  String get gameEngine;

  /// No description provided for @gameGeneric.
  ///
  /// In en, this message translates to:
  /// **'Generic'**
  String get gameGeneric;

  /// No description provided for @gameLods.
  ///
  /// In en, this message translates to:
  /// **'Levels of detail'**
  String get gameLods;

  /// No description provided for @gameLodsHint.
  ///
  /// In en, this message translates to:
  /// **'Percentages of the triangles, the first 100: for example 100, 50, 20. The edges of the pieces never move.'**
  String get gameLodsHint;

  /// No description provided for @gameNormalMap.
  ///
  /// In en, this message translates to:
  /// **'Normal map'**
  String get gameNormalMap;

  /// No description provided for @gameNoMap.
  ///
  /// In en, this message translates to:
  /// **'None'**
  String get gameNoMap;

  /// No description provided for @gameNormalMapHint.
  ///
  /// In en, this message translates to:
  /// **'The full garment\'s wrinkles, baked for each lighter level.'**
  String get gameNormalMapHint;

  /// No description provided for @gameSides.
  ///
  /// In en, this message translates to:
  /// **'Back faces'**
  String get gameSides;

  /// No description provided for @gameDoubleSided.
  ///
  /// In en, this message translates to:
  /// **'Double-sided'**
  String get gameDoubleSided;

  /// No description provided for @gameThickness.
  ///
  /// In en, this message translates to:
  /// **'Thickness'**
  String get gameThickness;

  /// No description provided for @gameThicknessMm.
  ///
  /// In en, this message translates to:
  /// **'Thickness'**
  String get gameThicknessMm;

  /// No description provided for @gameSkin.
  ///
  /// In en, this message translates to:
  /// **'Skin to the body\'s skeleton'**
  String get gameSkin;

  /// No description provided for @gameSkinHint.
  ///
  /// In en, this message translates to:
  /// **'The garment moves with the body, with the walk, turn and raise-arms motions as animations.'**
  String get gameSkinHint;

  /// No description provided for @gameSkinImported.
  ///
  /// In en, this message translates to:
  /// **'An imported body has no skeleton.'**
  String get gameSkinImported;

  /// No description provided for @gameAvatar.
  ///
  /// In en, this message translates to:
  /// **'Include the body'**
  String get gameAvatar;

  /// No description provided for @gameExport.
  ///
  /// In en, this message translates to:
  /// **'Export…'**
  String get gameExport;

  /// No description provided for @mm.
  ///
  /// In en, this message translates to:
  /// **'mm'**
  String get mm;

  /// No description provided for @errLods.
  ///
  /// In en, this message translates to:
  /// **'Levels of detail: start at 100 and go down, each above 0, at most five.'**
  String get errLods;

  /// No description provided for @errTextureSize.
  ///
  /// In en, this message translates to:
  /// **'Choose no map, or a power of two from 256 to 4096.'**
  String get errTextureSize;

  /// No description provided for @errNoSkeleton.
  ///
  /// In en, this message translates to:
  /// **'This body has no skeleton: export without skinning.'**
  String get errNoSkeleton;

  /// No description provided for @motionImport.
  ///
  /// In en, this message translates to:
  /// **'Import a captured motion…'**
  String get motionImport;

  /// No description provided for @motionFiles.
  ///
  /// In en, this message translates to:
  /// **'Motion captures (BVH)'**
  String get motionFiles;

  /// No description provided for @errMotionFormat.
  ///
  /// In en, this message translates to:
  /// **'This BVH file cannot be read: {detail}'**
  String errMotionFormat(String detail);

  /// No description provided for @errMotionSkeleton.
  ///
  /// In en, this message translates to:
  /// **'The skeleton of this motion has no joint for the {bone}.'**
  String errMotionSkeleton(String bone);

  /// No description provided for @errMotionTooShort.
  ///
  /// In en, this message translates to:
  /// **'This motion is too short: it needs at least {seconds} s.'**
  String errMotionTooShort(String seconds);

  /// No description provided for @motionLibrary.
  ///
  /// In en, this message translates to:
  /// **'Motion library'**
  String get motionLibrary;

  /// No description provided for @motionLibraryMenu.
  ///
  /// In en, this message translates to:
  /// **'Motion library…'**
  String get motionLibraryMenu;

  /// No description provided for @motionLibrarySubtitle.
  ///
  /// In en, this message translates to:
  /// **'Captured motions (BVH files) you can use in every project.'**
  String get motionLibrarySubtitle;

  /// No description provided for @motionLibraryEmpty.
  ///
  /// In en, this message translates to:
  /// **'No motion yet: add BVH files here, or add a motion to the library when you import it.'**
  String get motionLibraryEmpty;

  /// No description provided for @motionLibraryAdd.
  ///
  /// In en, this message translates to:
  /// **'Add files…'**
  String get motionLibraryAdd;

  /// No description provided for @motionLibraryPlay.
  ///
  /// In en, this message translates to:
  /// **'Play on the avatar'**
  String get motionLibraryPlay;

  /// No description provided for @motionLibraryDeleteConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete “{name}” from the library? Projects that use it keep their copy.'**
  String motionLibraryDeleteConfirm(String name);

  /// No description provided for @motionImported.
  ///
  /// In en, this message translates to:
  /// **'Motion imported into the project.'**
  String get motionImported;

  /// No description provided for @motionAddToLibrary.
  ///
  /// In en, this message translates to:
  /// **'Add to library'**
  String get motionAddToLibrary;

  /// No description provided for @motionAddedToLibrary.
  ///
  /// In en, this message translates to:
  /// **'Added to the library: {name}'**
  String motionAddedToLibrary(String name);

  /// No description provided for @errLibraryName.
  ///
  /// In en, this message translates to:
  /// **'“{name}” cannot name a motion: avoid / \\ : * ? \" < > | and a leading or trailing dot.'**
  String errLibraryName(String name);

  /// No description provided for @errLibraryExists.
  ///
  /// In en, this message translates to:
  /// **'The library already has a motion named “{name}”.'**
  String errLibraryExists(String name);

  /// No description provided for @errLibraryMissing.
  ///
  /// In en, this message translates to:
  /// **'The library has no motion named “{name}” any more.'**
  String errLibraryMissing(String name);

  /// No description provided for @errLibraryIo.
  ///
  /// In en, this message translates to:
  /// **'The motion library cannot be read or written: {detail}'**
  String errLibraryIo(String detail);
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
