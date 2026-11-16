// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class LFr extends L {
  LFr([String locale = 'fr']) : super(locale);

  @override
  String get languageTooltip => 'Langue';

  @override
  String get themeLight => 'Thème clair';

  @override
  String get themeDark => 'Thème sombre';

  @override
  String get cancel => 'Annuler';

  @override
  String get create => 'Créer';

  @override
  String get save => 'Enregistrer';

  @override
  String get apply => 'Appliquer';

  @override
  String get delete => 'Supprimer';

  @override
  String get next => 'Suivant';

  @override
  String get skip => 'Passer';

  @override
  String get start => 'Commencer';

  @override
  String get gotIt => 'Compris';

  @override
  String get cm => 'cm';

  @override
  String get keyCtrl => 'Ctrl';

  @override
  String get keyAlt => 'Alt';

  @override
  String get keyShift => 'Maj';

  @override
  String get keyDelete => 'Suppr';

  @override
  String get startTitle => 'Que voulez-vous créer ?';

  @override
  String get templateTShirt => 'T-shirt';

  @override
  String get templateTShirtBlurb =>
      'Devant, dos et manches courtes, en jersey.';

  @override
  String get templateDress => 'Robe';

  @override
  String get templateDressBlurb =>
      'Sans manches, légèrement cintrée, aux genoux.';

  @override
  String get templateStraightSkirt => 'Jupe droite';

  @override
  String get templateStraightSkirtBlurb =>
      'Près des hanches, de la taille aux genoux.';

  @override
  String get templateALineSkirt => 'Jupe trapèze';

  @override
  String get templateALineSkirtBlurb =>
      'S’évase doucement des hanches à l’ourlet.';

  @override
  String get templateToteBag => 'Sac cabas';

  @override
  String get templateToteBagBlurb =>
      'Deux pièces et quatre coutures : le plus simple pour commencer.';

  @override
  String get sectionNew => 'Nouveau';

  @override
  String get openFromDisk => 'Ouvrir…';

  @override
  String get open => 'Ouvrir';

  @override
  String allProjectsCount(int count) {
    return 'Tous les projets ($count)';
  }

  @override
  String get projectsFolder => 'Dossier des projets';

  @override
  String appVersion(String version) {
    return 'Version $version';
  }

  @override
  String get updatesLink => 'Mises à jour et actualités sur rustle3d.com';

  @override
  String cannotOpen(String target) {
    return 'Impossible d’ouvrir $target.';
  }

  @override
  String get blankProject => 'Projet vierge';

  @override
  String get blankProjectBlurb => 'Dessinez vos propres pièces.';

  @override
  String get untitled => 'Sans titre';

  @override
  String get recent => 'Récents';

  @override
  String get recentEmpty =>
      'Vos projets apparaîtront ici. Ils sont enregistrés au fil du travail, dans Documents › Rustle3D.';

  @override
  String get agoJustNow => 'à l’instant';

  @override
  String agoMinutes(int count) {
    return 'il y a $count min';
  }

  @override
  String agoHours(int count) {
    return 'il y a $count h';
  }

  @override
  String get agoYesterday => 'hier';

  @override
  String agoDays(int count) {
    return 'il y a $count jours';
  }

  @override
  String newGarmentTitle(String garment) {
    return 'Nouveau : $garment';
  }

  @override
  String get fieldName => 'Nom';

  @override
  String get fieldBody => 'Corps';

  @override
  String sizeMustBePositive(String size) {
    return '$size doit être un nombre de cm positif.';
  }

  @override
  String get sizeChest => 'Tour de poitrine';

  @override
  String get sizeChestHint =>
      'Autour de la partie la plus forte de la poitrine';

  @override
  String get sizeWaist => 'Tour de taille';

  @override
  String get sizeWaistHint => 'Autour de la partie la plus fine';

  @override
  String get sizeHips => 'Tour de hanches';

  @override
  String get sizeHipsHint => 'Autour de la partie la plus forte';

  @override
  String get sizeLength => 'Longueur';

  @override
  String get sizeLengthHint =>
      'Du cou à l’ourlet pour un haut, de la taille à l’ourlet pour une jupe, hauteur d’un sac';

  @override
  String get sizeSleeve => 'Manche';

  @override
  String get sizeSleeveHint => 'Depuis le haut de l’épaule';

  @override
  String get sizeWidth => 'Largeur';

  @override
  String get sizeWidthHint => 'Largeur du sac';

  @override
  String get sizeEase => 'Aisance';

  @override
  String get sizeEaseHint => 'Ampleur ajoutée au corps';

  @override
  String get pieceFront => 'Devant';

  @override
  String get pieceBack => 'Dos';

  @override
  String get pieceBackLeft => 'Dos gauche';

  @override
  String get pieceBackRight => 'Dos droit';

  @override
  String get pieceSleeveLeft => 'Manche gauche';

  @override
  String get pieceSleeveRight => 'Manche droite';

  @override
  String pieceNew(int number) {
    return 'Pièce $number';
  }

  @override
  String pieceMirror(String name) {
    return '$name (symétrique)';
  }

  @override
  String get body => 'Corps';

  @override
  String get bodySubtitle =>
      'Le corps 3D est ajusté à ces mesures. Partez d’une taille, puis ajustez.';

  @override
  String get bodyPreset => 'Modèle de corps';

  @override
  String presetWomen(String size) {
    return 'Femme $size';
  }

  @override
  String presetMen(String size) {
    return 'Homme $size';
  }

  @override
  String get bodyFeminine => 'Corps féminin';

  @override
  String get bodyMasculine => 'Corps masculin';

  @override
  String get measHeight => 'Stature';

  @override
  String get measBust => 'Tour de poitrine';

  @override
  String measBetween(String measure, int min, int max) {
    return '$measure doit être entre $min et $max cm.';
  }

  @override
  String get presetName => 'Nom du modèle';

  @override
  String get saveAsPreset => 'Enregistrer comme modèle';

  @override
  String get deletePreset => 'Supprimer le modèle';

  @override
  String fittedBody(String height, String bust, String waist, String hips) {
    return 'Corps ajusté : hauteur $height · poitrine $bust · taille $waist · hanches $hips';
  }

  @override
  String get allProjects => 'Tous les projets';

  @override
  String get export => 'Exporter';

  @override
  String get agents => 'Agents';

  @override
  String get agentsWaiting => 'Agents : en attente';

  @override
  String get agentConnected => 'Agent connecté';

  @override
  String get agentsTip =>
      'Agents IA : laisser un agent modifier ce projet en direct';

  @override
  String get allowAgents => 'Autoriser les agents IA';

  @override
  String get agentsHow => 'Un agent se connecte avec : r3d mcp --attach';

  @override
  String get disconnectAgent => 'Déconnecter l’agent';

  @override
  String get exportSvg => 'Patron, SVG à l’échelle 1';

  @override
  String get exportPdfA4 => 'Patron, PDF en pages A4';

  @override
  String get exportPdfSingle => 'Patron, PDF sur une seule page';

  @override
  String get exportGlb => 'Vêtement en 3D, glTF (.glb)';

  @override
  String get exportGlbAvatar => 'Vêtement et corps en 3D, glTF (.glb)';

  @override
  String get exportObj => 'Vêtement en 3D, OBJ';

  @override
  String exported(String file) {
    return 'Exporté : $file.';
  }

  @override
  String get learn => 'Apprendre';

  @override
  String get undoTip => 'Annuler  ⌘Z';

  @override
  String get redoTip => 'Rétablir  ⇧⌘Z';

  @override
  String get tourPatternTitle => 'Votre patron';

  @override
  String get tourPatternText =>
      'Les pièces à plat, en taille réelle. Faites glisser un coin ou un bord pour les modifier ; **R** ajoute une pièce.';

  @override
  String get tourGarmentTitle => 'Le vêtement sur le corps';

  @override
  String get tourGarmentText =>
      '**Simuler** le fait tomber sur le corps. Faites glisser pour tourner autour ; **⇧-glisser** un ourlet ou une couture modifie le patron depuis ici.';

  @override
  String get tourGuideTitle => 'Une étape à la fois';

  @override
  String get tourGuideText =>
      'Le guide indique quoi faire ensuite. Cliquez sur une étape en haut pour y aller, ou fermez le guide.';

  @override
  String get tourHelpTitle => 'L’aide est toujours là';

  @override
  String get tourHelpText =>
      '**Apprendre** propose de courts tutoriels et montre où se trouvent les choses ; **Corps** change les mesures. **⌘Z** annule tout.';

  @override
  String get stepMeasurements => 'Mesures';

  @override
  String get stepMeasurementsGoal =>
      'Vérifiez le corps pour lequel le vêtement est fait : sa taille et ses mesures.';

  @override
  String get stepMeasurementsAction => 'Ouvrir Corps';

  @override
  String get stepPattern => 'Patron';

  @override
  String get stepPatternGoal =>
      'Façonnez les pièces : faites glisser un coin ou un bord, ou cliquez un bord pour taper sa longueur.';

  @override
  String get stepPatternAction => 'Ajouter une pièce';

  @override
  String get stepSew => 'Coudre';

  @override
  String get stepSewGoal =>
      'Assemblez les pièces : cliquez un bord sur le patron, puis le bord auquel le coudre.';

  @override
  String get stepSewAction => 'Outil Coudre';

  @override
  String get stepFabric => 'Tissu';

  @override
  String get stepFabricGoal =>
      'Choisissez la matière du vêtement, dans le panneau de droite.';

  @override
  String get stepDrape => 'Tombé';

  @override
  String get stepDrapeGoal =>
      'Regardez-le tomber sur le corps. Puis tirez le tissu, ou ⌥-clic pour l’épingler.';

  @override
  String stepOf(int step, int total) {
    return 'Étape $step sur $total';
  }

  @override
  String get guideNext => 'Suivant  →';

  @override
  String get guideHide =>
      'Masquer le guide (cliquez une étape pour le rouvrir)';

  @override
  String get toolSelect => 'Sélection';

  @override
  String get toolSelectHint =>
      'Cliquez une pièce ; faites glisser un coin ou un bord pour la modifier';

  @override
  String get toolRectangle => 'Rectangle';

  @override
  String get toolRectangleHint => 'Ajouter une pièce rectangulaire';

  @override
  String get toolPolygon => 'Polygone';

  @override
  String get toolCurve => 'Courbe';

  @override
  String get toolDraw => 'Dessiner sur le vêtement';

  @override
  String get toolDrawHint =>
      'Faites glisser sur le vêtement 3D : la ligne va sur le patron';

  @override
  String get toolNotch => 'Cran';

  @override
  String get toolSew => 'Coudre';

  @override
  String get toolSewHint => 'Cliquez deux bords sur le patron';

  @override
  String get comingSoon => 'Bientôt';

  @override
  String get viewPattern => 'Patron';

  @override
  String get view3d => '3D';

  @override
  String get hintSewFirst =>
      'Coudre : cliquez le premier bord sur le patron · V pour arrêter';

  @override
  String get hintSewSecond =>
      'Coudre : cliquez maintenant le bord à y coudre · V pour arrêter';

  @override
  String get hintDraw =>
      'Dessin : faites glisser sur le vêtement 3D ; la ligne va sur le patron · V pour arrêter';

  @override
  String get hintIdle =>
      'Patron : glissez un coin ou un bord, cliquez un bord pour sa longueur · 3D : glissez pour tirer le tissu · ⇧-glisser un ourlet ou une couture · ⌘-glisser pour déplacer une pièce · ⌥-clic pour épingler';

  @override
  String get hintSelection =>
      'Réglez la sélection dans l’inspecteur · ⌫ supprime · ⌘Z annule';

  @override
  String statsSim(int points, String stepMs, String time) {
    return 'Tissu $points points · pas $stepMs ms · $time s · ';
  }

  @override
  String statsRender(String boardMs, String garmentMs, String fps) {
    return 'Rendu Rust · patron $boardMs ms · 3D $garmentMs ms · $fps i/s';
  }

  @override
  String get toastSewn =>
      'Cousu. Inversez-le dans l’inspecteur s’il paraît vrillé.';

  @override
  String get toastPinned => 'Épinglé. Retirez l’épingle dans l’inspecteur.';

  @override
  String get toastLineAdded => 'Ligne ajoutée au patron.';

  @override
  String deleteTitle(String name) {
    return 'Supprimer « $name » ?';
  }

  @override
  String get deleteUndo => 'Vous pourrez annuler avec ⌘Z.';

  @override
  String get camFront => 'Face';

  @override
  String get camSide => 'Profil';

  @override
  String get camBack => 'Dos';

  @override
  String get camFrontTip => 'Vue de face  1';

  @override
  String get camSideTip => 'Vue de profil  2';

  @override
  String get camBackTip => 'Vue de dos  3';

  @override
  String get camThreeQuarterTip => 'Vue de trois quarts  4';

  @override
  String get simulate => 'Simuler';

  @override
  String get pause => 'Pause';

  @override
  String get simulateTip => 'Faire tomber le vêtement  Espace';

  @override
  String get pauseTip => 'Pause  Espace';

  @override
  String get restartTip => 'Recommencer depuis le patron';

  @override
  String get inspector => 'Inspecteur';

  @override
  String get selectPanel => 'Sélectionnez une pièce';

  @override
  String selectPanelHint(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pièces',
      one: '1 pièce',
      zero: 'Aucune pièce',
    );
    return 'Cliquez une pièce sur le patron ou le vêtement\npour la régler ici. $_temp0 dans ce projet.';
  }

  @override
  String get arrange => 'Disposer autour du corps';

  @override
  String get arrangeHint =>
      'Place chaque pièce autour du corps dans l’ordre des coutures, à la taille.';

  @override
  String get pins => 'Épingles';

  @override
  String get pinsEmpty =>
      '⌥-clic sur le tissu en 3D pour le maintenir en place.';

  @override
  String get pinsAdd => '⌥-clic sur le tissu en 3D pour en ajouter une.';

  @override
  String get removePin => 'Retirer cette épingle';

  @override
  String get wholeGarment => 'Vêtement entier';

  @override
  String piecesSelected(int count) {
    return '$count pièces sélectionnées. Cliquez une seule pièce sur le patron pour la modifier.';
  }

  @override
  String get piece => 'Pièce';

  @override
  String get mirrorCopy => 'Copie symétrique — suit sa source';

  @override
  String get unlink => 'Délier';

  @override
  String get area => 'Surface';

  @override
  String get perimeter => 'Périmètre';

  @override
  String get seamAllowance => 'Valeur de couture';

  @override
  String get grainline => 'Droit-fil';

  @override
  String get mirror => 'Symétrique';

  @override
  String get onTheBody => 'Sur le corps';

  @override
  String get notSimulated =>
      'Pas encore simulée : placez-la pour la voir en 3D.';

  @override
  String get placeOnBody => 'Placer sur le corps';

  @override
  String get aroundBody => 'Corps';

  @override
  String get aroundLeftArm => 'Bras gauche';

  @override
  String get aroundRightArm => 'Bras droit';

  @override
  String get aroundFlat => 'À plat';

  @override
  String get aroundAngle => 'Autour (0 = devant)';

  @override
  String get aroundArmAngle => 'Autour du bras (0 = extérieur)';

  @override
  String get topHeight => 'Hauteur du haut';

  @override
  String get fabric => 'Tissu';

  @override
  String get fabricCotton => 'Coton';

  @override
  String get fabricCottonBlurb => 'Net et respirant';

  @override
  String get fabricDenim => 'Denim';

  @override
  String get fabricDenimBlurb => 'Lourd, garde sa forme';

  @override
  String get fabricJersey => 'Jersey';

  @override
  String get fabricJerseyBlurb => 'Maille extensible';

  @override
  String get fabricSilk => 'Soie';

  @override
  String get fabricSilkBlurb => 'Légère et fluide';

  @override
  String get fabricWool => 'Laine';

  @override
  String get fabricWoolBlurb => 'Douce, avec de la tenue';

  @override
  String edgeLength(String length) {
    return 'Bord · $length cm';
  }

  @override
  String get edgeOpen =>
      'Bord libre (un ourlet ou une ouverture). En 3D, ⇧-glissez-le pour rallonger le vêtement.';

  @override
  String get edgeLengthLabel => 'Longueur';

  @override
  String edgeSewn(String piece, String mismatch) {
    return 'Cousu à $piece · $mismatch mm d’écart. Sa longueur change des deux côtés. En 3D, ⇧-glissez-le pour le reprendre.';
  }

  @override
  String get matchOtherSide => 'Mettre l’autre côté à cette longueur';

  @override
  String get gathered => 'Froncée';

  @override
  String gatheredHint(String ratio) {
    return 'Le côté le plus long est froncé régulièrement sur le plus court ($ratio : 1).';
  }

  @override
  String get flipSeam => 'Inverser la couture';

  @override
  String get unpick => 'Découdre';

  @override
  String changeTakenIn(String amount) {
    return 'Repris de $amount mm';
  }

  @override
  String changeLetOut(String amount) {
    return 'Relâché de $amount mm';
  }

  @override
  String changeLonger(String amount) {
    return 'Rallongé de $amount mm';
  }

  @override
  String changeMoved(String amount) {
    return 'Déplacé de $amount mm';
  }

  @override
  String changeShorter(String amount) {
    return 'Raccourci de $amount mm';
  }

  @override
  String get learnIntro =>
      'De courts tutoriels. « Montrer » indique où cela se passe ; votre projet n’est pas modifié.';

  @override
  String get showMe => 'Montrer';

  @override
  String get learnDrape => 'Faire tomber le vêtement';

  @override
  String get learnDrapeHow =>
      'Appuyez sur **Simuler** (**Espace**) : les pièces ferment leurs coutures et tombent sur le corps.';

  @override
  String get learnFabric => 'Essayer un autre tissu';

  @override
  String get learnFabricHow =>
      'Sans rien sélectionner, choisissez un échantillon sous **Tissu**. Le tissu change le tombé du vêtement.';

  @override
  String get learnLength => 'Changer une longueur';

  @override
  String get learnLengthHow =>
      'Cliquez un bord (une couture de côté pour rallonger, un ourlet pour élargir), puis tapez sa **Longueur** ou faites glisser le curseur. Ou maintenez **⇧** et faites glisser l’ourlet sur le vêtement.';

  @override
  String get learnFit => 'Reprendre une couture';

  @override
  String get learnFitHow =>
      'Maintenez **⇧** et faites glisser une couture sur le vêtement : vers le corps la reprend, vers l’extérieur la relâche.';

  @override
  String get learnSew => 'Coudre deux pièces';

  @override
  String get learnSewHow =>
      'Avec **Coudre** (**S**), cliquez un bord sur le patron, puis le bord auquel le coudre.';

  @override
  String get learnDraw => 'Tracer une ligne de style';

  @override
  String get learnDrawHow =>
      'Avec **Dessiner sur le vêtement** (**D**), faites glisser sur le vêtement 3D : la ligne arrive sur le patron.';

  @override
  String get learnMove => 'Déplacer une pièce autour du corps';

  @override
  String get learnMoveHow =>
      'Maintenez **⌘** et faites glisser une pièce sur le vêtement, ou réglez-la dans l’inspecteur.';

  @override
  String get learnPin => 'Tirer et épingler le tissu';

  @override
  String get learnPinHow =>
      'Faites glisser le tissu en 3D pour le tirer. **⌥-clic** l’épingle là où il est.';

  @override
  String get learnLook => 'Regarder autour';

  @override
  String get learnLookHow =>
      'Faites glisser pour tourner autour du corps ; molette ou pincement pour zoomer. Touches **1**–**4** : face, profil, dos, trois quarts.';

  @override
  String get learnBody => 'Habiller un autre corps';

  @override
  String get learnBodyHow =>
      'Ouvrez **Corps** : choisissez une taille ou entrez des mesures, et le vêtement tombe dessus.';

  @override
  String get learnDrapeStop => 'Simuler';

  @override
  String get learnDrapeStopText =>
      'Appuyez sur **Simuler** (**Espace**) pour faire tomber le vêtement, à nouveau pour mettre en pause. La flèche à côté recommence depuis le patron.';

  @override
  String get learnFabricStop => 'Tissu';

  @override
  String get learnFabricStopText =>
      'Sans rien sélectionner, le panneau de droite montre le vêtement : choisissez un échantillon sous **Tissu**. Le tombé suit.';

  @override
  String get learnPickEdge => 'Choisir un bord';

  @override
  String get learnPickEdgeText =>
      'Cliquez un bord sur le patron, ou sur le vêtement.';

  @override
  String get learnEdgeLength => 'Sa longueur';

  @override
  String get learnEdgeLengthText =>
      'Tapez la nouvelle **Longueur** puis **Entrée**, ou faites glisser le curseur. L’autre côté d’un bord cousu suit, les coutures restent alignées.';

  @override
  String get learnDragHem => 'Ou faites-le glisser';

  @override
  String get learnDragHemText =>
      'Maintenez **⇧** et faites glisser l’ourlet sur le vêtement : vers le bas rallonge, vers le haut raccourcit.';

  @override
  String get learnDragSeam => 'Ou faites-la glisser';

  @override
  String get learnDragSeamText =>
      'Maintenez **⇧** et faites glisser une couture sur le vêtement : vers le corps la reprend, vers l’extérieur la relâche. Près d’une extrémité, seule cette extrémité bouge.';

  @override
  String get learnSewTool => 'Outil Coudre';

  @override
  String get learnSewToolText =>
      'Choisissez **Coudre** ici, ou appuyez sur **S**.';

  @override
  String get learnTwoEdges => 'Cliquez deux bords';

  @override
  String get learnTwoEdgesText =>
      'Cliquez un bord, puis le bord auquel le coudre. Des flèches assorties montrent comment ils se joignent.';

  @override
  String get learnDrawTool => 'Dessiner sur le vêtement';

  @override
  String get learnDrawToolText =>
      'Choisissez **Dessiner sur le vêtement** ici, ou appuyez sur **D**.';

  @override
  String get learnDrawAcross => 'Faites glisser sur le vêtement';

  @override
  String get learnDrawAcrossText =>
      'La ligne arrive sur le patron, pour un empiècement ou une poche.';

  @override
  String get learnDragPiece => 'Faites glisser une pièce';

  @override
  String get learnDragPieceText =>
      'Maintenez **⌘** et faites glisser une pièce : de côté la fait tourner autour du corps, de haut en bas change sa hauteur.';

  @override
  String get learnSetPlacement => 'Ou réglez-la ici';

  @override
  String get learnSetPlacementText =>
      'Sélectionnez la pièce : **Sur le corps** offre les mêmes réglages, en valeurs à saisir.';

  @override
  String get learnPullPin => 'Tirer et épingler';

  @override
  String get learnPullPinText =>
      'Faites glisser le tissu pour le tirer. **⌥-clic** l’épingle là où il est.';

  @override
  String get learnYourPins => 'Vos épingles';

  @override
  String get learnYourPinsText =>
      'Sans rien sélectionner, les épingles sont listées sous **Épingles** ; **×** en retire une.';

  @override
  String get learnViews => 'Vues';

  @override
  String get learnViewsText =>
      '**Face**, **Profil**, **Dos** et **¾**, ou les touches **1**–**4**.';

  @override
  String get learnTurnZoom => 'Tourner et zoomer';

  @override
  String get learnTurnZoomText =>
      'Faites glisser autour du corps pour tourner ; molette, ou pincement sur le trackpad, pour zoomer.';

  @override
  String get learnBodyStop => 'Corps';

  @override
  String get learnBodyStopText =>
      'Choisissez une taille ou entrez des mesures : le corps est réajusté et le vêtement tombe dessus.';

  @override
  String errNotFound(String id) {
    return '$id n’existe pas.';
  }

  @override
  String errLinkedMirror(String panel) {
    return '$panel est une copie symétrique liée : modifiez sa source, ou déliez-la d’abord.';
  }

  @override
  String get errSewn => 'Ce bord est cousu : décousez d’abord la couture.';

  @override
  String get errNothingToUndo => 'Rien à annuler.';

  @override
  String get errNothingToRedo => 'Rien à rétablir.';

  @override
  String get errNothingToSimulate =>
      'Rien à simuler : placez au moins une pièce sur le corps.';

  @override
  String get errInverted => 'Cela retournerait une pièce sur elle-même.';

  @override
  String errRange(String min, String max) {
    return 'La valeur doit être entre $min et $max.';
  }

  @override
  String get errNotPositive => 'La valeur doit être supérieure à zéro.';

  @override
  String get errNegative => 'La valeur ne doit pas être négative.';

  @override
  String get errEmpty => 'Le nom ne doit pas être vide.';

  @override
  String get errNotSimple => 'Le contour ne doit pas se croiser.';

  @override
  String get errFileIo =>
      'Impossible de lire ou d’écrire les fichiers du projet.';

  @override
  String get errNotAProject => 'Ce dossier n’est pas un projet Rustle3D.';

  @override
  String get errTooNew =>
      'Ce projet a été créé par une version plus récente de Rustle3D.';

  @override
  String get errCorrupted => 'Ce fichier de projet est endommagé.';

  @override
  String get errAlreadyExists => 'Un projet existe déjà à cet endroit.';
}
