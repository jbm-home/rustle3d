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
  String get templateShirt => 'Chemise';

  @override
  String get templateShirtBlurb =>
      'Manches longues à poignets et col officier.';

  @override
  String get templateTrousers => 'Pantalon';

  @override
  String get templateTrousersBlurb =>
      'Jambes droites, de la taille à la cheville.';

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
  String get importProjectZip => 'Importer un .zip…';

  @override
  String get importProjectZipTip =>
      'Ajouter un projet reçu sous forme d’archive .zip';

  @override
  String get projectZipFiles => 'Archives de projet';

  @override
  String get projectOptions => 'Options du projet';

  @override
  String get renameProject => 'Renommer…';

  @override
  String get exportProjectZip => 'Exporter en .zip…';

  @override
  String get deleteProject => 'Supprimer…';

  @override
  String get renameProjectTitle => 'Renommer le projet';

  @override
  String get rename => 'Renommer';

  @override
  String deleteProjectTitle(String name) {
    return 'Supprimer « $name » ?';
  }

  @override
  String get deleteProjectBody =>
      'Le dossier du projet et tout son contenu (patron, corps, exports) seront supprimés de cet ordinateur. Cette action est irréversible.';

  @override
  String get deleteProjectConfirm =>
      'Je comprends que ce projet sera supprimé définitivement';

  @override
  String projectDeleted(String name) {
    return '« $name » supprimé.';
  }

  @override
  String projectExported(String name, String file) {
    return '« $name » exporté dans $file.';
  }

  @override
  String appVersion(String version) {
    return 'Version $version';
  }

  @override
  String get updatesLink => 'Mises à jour et actualités sur rustle3d.com';

  @override
  String get supportLink => 'Soutenir Rustle3D';

  @override
  String get supportTitle => 'Rustle3D est libre et gratuit';

  @override
  String get supportText =>
      'S’il vous est utile, un don aide à couvrir ses frais : hébergement, matériel de test et temps de développement. Tout reste gratuit dans tous les cas.';

  @override
  String get supportDonate => 'Faire un don';

  @override
  String get supportLater => 'Pas maintenant';

  @override
  String get supportTurnOff => 'Ne plus afficher';

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
  String get importPattern => 'Importer';

  @override
  String get importTip =>
      'Importer un patron dessiné ailleurs : DXF-AAMA, DXF, SVG ou PDF';

  @override
  String get importFiles => 'Patrons';

  @override
  String imported(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pièces importées',
      one: '1 pièce importée',
    );
    return '$_temp0.';
  }

  @override
  String importIgnored(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count tracés ne faisaient partie d’aucune pièce.',
      one: '1 tracé ne faisait partie d’aucune pièce.',
    );
    return '$_temp0';
  }

  @override
  String get importUnitsTitle => 'Dans quelle unité ce patron est-il dessiné ?';

  @override
  String get importUnitsBody =>
      'Le fichier n’indique pas son échelle : les pièces n’ont peut-être pas la bonne taille. Choisissez l’unité du dessin, ou gardez-les telles quelles.';

  @override
  String get importKeep => 'Garder tel quel';

  @override
  String get unitMm => 'Millimètres';

  @override
  String get unitCm => 'Centimètres';

  @override
  String get unitIn => 'Pouces';

  @override
  String get unitPt => 'Points (1/72 de pouce)';

  @override
  String get unitPx => 'Pixels (1/96 de pouce)';

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
  String get exportDxf => 'Patron, DXF-AAMA (logiciels de patronage)';

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
  String get toolPolygonHint => 'Cliquez les coins d’une nouvelle pièce';

  @override
  String get toolCurve => 'Courbe';

  @override
  String get toolCurveHint =>
      'Tracez une pièce : cliquez pour un coin, glissez pour un point arrondi';

  @override
  String get toolEditCurve => 'Modifier les courbes';

  @override
  String get toolEditCurveHint =>
      'Courber un bord, le redresser ou y ajouter un point';

  @override
  String get toolBend => 'Courber';

  @override
  String get toolBendHint =>
      'Cliquez un bord, puis faites glisser ses poignées pour le courber';

  @override
  String get toolStraighten => 'Redresser';

  @override
  String get toolStraightenHint =>
      'Cliquez un bord courbe pour le rendre droit';

  @override
  String get toolAddPoint => 'Ajouter un point';

  @override
  String get toolAddPointHint => 'Cliquez un bord là où va le nouveau point';

  @override
  String get toolCut => 'Couper';

  @override
  String get toolCutHint =>
      'Tracez une ligne à travers une pièce : elle devient deux pièces cousues';

  @override
  String get toolDraw => 'Dessiner sur le vêtement';

  @override
  String get toolDrawHint =>
      'Faites glisser sur le vêtement 3D : la ligne va sur le patron';

  @override
  String get toolNotch => 'Cran';

  @override
  String get toolNotchHint => 'Cliquez un bord à l’endroit du cran';

  @override
  String get toolSew => 'Coudre';

  @override
  String get toolSewHint =>
      'Cliquez deux bords ou lignes sur le patron ; glissez le long d’un bord pour n’en coudre qu’une partie';

  @override
  String get viewPattern => 'Patron';

  @override
  String get view3d => '3D';

  @override
  String get hintPolygon =>
      'Polygone : cliquez les coins ; cliquez le premier ou Entrée pour fermer · Échap annule · V pour arrêter';

  @override
  String get hintCurve =>
      'Courbe : cliquez pour un coin, glissez pour un point arrondi ; cliquez le premier point ou Entrée pour fermer · Échap annule · V pour arrêter';

  @override
  String get hintBend =>
      'Courber : cliquez un bord, puis faites glisser ses poignées · V pour arrêter';

  @override
  String get hintStraighten =>
      'Redresser : cliquez un bord courbe · V pour arrêter';

  @override
  String get hintAddPoint =>
      'Ajouter un point : cliquez un bord là où va le point · V pour arrêter';

  @override
  String get hintCut =>
      'Couper : cliquez à travers une pièce, glissez pour une courbe, de l’extérieur à l’extérieur ; Entrée coupe · Échap annule · V pour arrêter';

  @override
  String get hintNotch =>
      'Cran : cliquez un bord à l’endroit du cran · V pour arrêter';

  @override
  String get toastPieceAdded => 'Pièce ajoutée.';

  @override
  String get toastNotchAdded => 'Cran ajouté.';

  @override
  String get toastCut => 'Coupée en deux pièces, cousues ensemble.';

  @override
  String get toastPointAdded => 'Point ajouté.';

  @override
  String get distance => 'Distance';

  @override
  String get pointPicked =>
      'Coin choisi. ⌘-cliquez un autre coin pour mesurer la distance entre eux.';

  @override
  String get pointsPicked =>
      'Entre les deux coins. Le second se déplace le long de la ligne.';

  @override
  String get straighten => 'Redresser';

  @override
  String get hintSewFirst =>
      'Coudre : cliquez un bord ou une ligne, ou glissez le long pour n’en coudre qu’une partie · ⌘-cliquez des bords qui se suivent pour les coudre comme un seul côté · V pour arrêter';

  @override
  String get hintSewSecond =>
      'Coudre : maintenant le côté à y coudre : cliquez-le, glissez le long d’une partie ou ⌘-cliquez plusieurs bords · Échap pour recommencer';

  @override
  String get hintSewRun =>
      'Coudre : ⌘-cliquez les bords suivants, puis cliquez le dernier ou appuyez sur Entrée · Échap pour recommencer';

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
  String statsSim(int points, String backend, String stepMs, String time) {
    return 'Tissu $points points · pas $backend $stepMs ms · $time s · ';
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
  String get maximizeViewTip => 'Agrandir à la place des deux vues  M';

  @override
  String get restoreViewsTip => 'Revenir aux deux vues  M';

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
  String get aroundLeftLeg => 'Jambe gauche';

  @override
  String get aroundRightLeg => 'Jambe droite';

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
  String get garments => 'Vêtements';

  @override
  String get garmentsHint =>
      'Portés du corps vers l’extérieur : un vêtement se drape par-dessus ceux qui le précèdent dans la liste.';

  @override
  String get garmentNextToBody => 'Contre le corps';

  @override
  String get garmentOver => 'Porté par-dessus le précédent';

  @override
  String get garmentMoveIn => 'Le porter dessous';

  @override
  String get garmentMoveOut => 'Le porter dessus';

  @override
  String get garmentRemove => 'Supprimer le vêtement et ses pièces';

  @override
  String get addGarment => 'Ajouter un vêtement';

  @override
  String garmentNewName(int number) {
    return 'Vêtement $number';
  }

  @override
  String get pieceGarment => 'Vêtement';

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
  String get errNotAcross =>
      'La coupe doit traverser une pièce de part en part : commencez et finissez à l’extérieur.';

  @override
  String get errMirrored =>
      'Cette pièce a une copie miroir liée : déliez d’abord la copie.';

  @override
  String get errLastGarment => 'Un projet garde au moins un vêtement.';

  @override
  String get errFileIo =>
      'Impossible de lire ou d’écrire les fichiers du projet.';

  @override
  String errImportMalformed(String format) {
    return 'Ce fichier ne peut pas être lu comme un $format.';
  }

  @override
  String get errImportBinaryDxf =>
      'Les fichiers DXF binaires ne peuvent pas être lus : enregistrez le fichier en DXF ASCII.';

  @override
  String get errImportNoPieces =>
      'Aucune pièce de patron : le dessin n’a pas de contour fermé.';

  @override
  String get errImportPagesDiffer =>
      'Les pages de ce PDF ne reprennent pas un même dessin : elles ne peuvent pas être réassemblées.';

  @override
  String get errImportEncrypted => 'Ce PDF est protégé et ne peut pas être lu.';

  @override
  String get errReadFile => 'Impossible de lire ce fichier.';

  @override
  String get errNotAProject => 'Ce dossier n’est pas un projet Rustle3D.';

  @override
  String get errTooNew =>
      'Ce projet a été créé par une version plus récente de Rustle3D.';

  @override
  String get errCorrupted => 'Ce fichier de projet est endommagé.';

  @override
  String get errAlreadyExists => 'Un projet existe déjà à cet endroit.';

  @override
  String get errArchiveInvalid =>
      'Ce fichier n’est pas une archive .zip lisible.';

  @override
  String get errArchiveNoProject =>
      'Cette archive ne contient pas de projet Rustle3D.';

  @override
  String errArchiveTooLarge(String limit) {
    return 'Cette archive est trop volumineuse pour être importée (plus de $limit Mo une fois décompressée).';
  }

  @override
  String get photo => 'Photo';

  @override
  String get photoTip => 'Faire une image photoréaliste du vêtement';

  @override
  String get photoTitle => 'Faire une photo';

  @override
  String get photoIntro =>
      'Une image photoréaliste du vêtement tel que la vue 3D le montre maintenant, dans un studio photo.';

  @override
  String get photoQuality => 'Qualité';

  @override
  String get photoQuick => 'Rapide';

  @override
  String get photoQuickHint => 'Un premier aperçu en quelques secondes';

  @override
  String get photoGood => 'Bonne';

  @override
  String get photoGoodHint => 'Assez nette pour être montrée';

  @override
  String get photoBest => 'Maximale';

  @override
  String get photoBestHint => 'Pour l\'impression et les gros plans';

  @override
  String get photoLighting => 'Éclairage';

  @override
  String get lightStudio => 'Studio';

  @override
  String get lightStudioHint =>
      'Lumière principale, d\'appoint et de contre-jour sur fond gris clair';

  @override
  String get lightSoft => 'Doux';

  @override
  String get lightSoftHint => 'Grandes lumières douces sur fond blanc';

  @override
  String get lightDramatic => 'Contrasté';

  @override
  String get lightDramaticHint => 'Une lumière dure sur fond sombre';

  @override
  String get photoSize => 'Taille';

  @override
  String get photoSizeView => 'Comme la vue 3D';

  @override
  String get photoAdvanced => 'Avancé';

  @override
  String get photoSamples => 'Échantillons par pixel';

  @override
  String get photoBounces => 'Rebonds de lumière';

  @override
  String get photoDenoise => 'Retirer le bruit';

  @override
  String get photoRender => 'Lancer le rendu';

  @override
  String photoRendering(int done, int total, String seconds) {
    return 'Rendu en cours… $done / $total échantillons · $seconds s';
  }

  @override
  String photoDone(String seconds) {
    return 'Terminé en $seconds s';
  }

  @override
  String photoStopped(int done) {
    return 'Arrêté à $done échantillons';
  }

  @override
  String photoFailed(String reason) {
    return 'La photo n\'a pas pu être rendue : $reason';
  }

  @override
  String get photoStop => 'Arrêter';

  @override
  String get photoStopTip =>
      'Arrêter d\'affiner et garder l\'image telle quelle';

  @override
  String get photoSave => 'Enregistrer l\'image…';

  @override
  String get photoClose => 'Fermer';

  @override
  String get photoCloseTip => 'Revenir à la vue 3D en direct';

  @override
  String photoSaved(String file) {
    return '$file enregistré.';
  }

  @override
  String get errPhotoTooLarge =>
      'Cette image est trop grande. Choisissez une taille plus petite (3840 × 2160 au plus).';

  @override
  String get errNoPhoto => 'Il n\'y a pas de photo à enregistrer.';

  @override
  String get learnPhoto => 'Faire une photo';

  @override
  String get learnPhotoHow =>
      'Une image photoréaliste du vêtement, à montrer à un client ou à publier.';

  @override
  String get learnPhotoStop => 'Photo';

  @override
  String get learnPhotoStopText =>
      'Tournez la vue 3D sous l\'angle qui vous plaît, cliquez sur le bouton **appareil photo**, choisissez la qualité et l\'éclairage, puis **Lancer le rendu**. L\'image s\'affine dans la vue 3D ; **Enregistrer l\'image…** la garde en PNG.';

  @override
  String get sizes => 'Tailles';

  @override
  String get sizesHint =>
      'Gradez le patron sur une gamme de tailles. Il reste tracé dans la taille de base ; les contours des autres tailles s\'emboîtent autour de chaque pièce.';

  @override
  String get addSizes => 'Ajouter une gamme de tailles';

  @override
  String get sizeChart => 'Barème';

  @override
  String get chartWomen => 'Femme';

  @override
  String get chartMen => 'Homme';

  @override
  String get sizeFrom => 'De';

  @override
  String get sizeTo => 'À';

  @override
  String get sizeBase => 'Taille de base';

  @override
  String get sizesDialogHint =>
      'Les pièces ajoutées ensuite depuis un modèle sont gradées dans toutes les tailles. Chaque taille est drapée sur un corps à ses mesures.';

  @override
  String get sizeShown => 'Affichée en 3D';

  @override
  String get sizesChange => 'Modifier';

  @override
  String get sizesRemove => 'Retirer les tailles';

  @override
  String get sizeBaseTag => 'base';

  @override
  String get gradeStep => 'Gradation par taille';

  @override
  String get gradeStepHint =>
      'De combien ce point bouge d\'une taille à la suivante.';

  @override
  String get gradeCustom =>
      'Réglée taille par taille (par un modèle ou un agent). Un pas saisi ici la remplace.';

  @override
  String get errNoSizes => 'Ajoutez d\'abord une gamme de tailles.';

  @override
  String get errUnknownSize => 'Cette taille n\'est pas dans la gamme.';

  @override
  String get errTooFewSizes => 'Une gamme compte au moins deux tailles.';

  @override
  String get errDuplicateName => 'Deux tailles portent le même nom.';

  @override
  String get errBodyOutOfRange =>
      'Une taille de cette gamme donnerait à l\'avatar des mesures hors des limites acceptées.';

  @override
  String get errBaseOffset =>
      'La taille de base ne bouge pas : c\'est le patron tel qu\'il est tracé.';

  @override
  String get sizesOnBoard => 'Afficher les tailles sur le plateau';

  @override
  String stepName(String step) {
    String _temp0 = intl.Intl.selectLogic(step, {
      'project_rename': 'Renommer le projet',
      'project_set_display_unit': 'Changer d’unité',
      'avatar_set': 'Modifier le corps',
      'avatar_remove': 'Retirer le corps',
      'panel_create_rect': 'Ajouter un rectangle',
      'panel_create_polygon': 'Ajouter un polygone',
      'panel_create_path': 'Ajouter une pièce courbe',
      'panel_rename': 'Renommer la pièce',
      'panel_translate': 'Déplacer la pièce',
      'panel_set_grain': 'Régler le droit-fil',
      'panel_set_seam_allowance': 'Régler la valeur de couture',
      'panel_mirror': 'Symétrie de la pièce',
      'panel_unlink': 'Délier la copie',
      'panel_place': 'Placer la pièce',
      'panel_delete': 'Supprimer la pièce',
      'panel_cut': 'Couper la pièce',
      'panel_paste': 'Coller',
      'point_move': 'Déplacer le point',
      'point_set_distance': 'Fixer la distance',
      'point_remove': 'Retirer le point',
      'edge_set_curve': 'Courber le bord',
      'edge_split': 'Ajouter un point',
      'edge_move': 'Déplacer le bord',
      'notch_add': 'Ajouter un cran',
      'notch_move': 'Déplacer le cran',
      'notch_remove': 'Retirer le cran',
      'internal_line_add': 'Tracer une ligne',
      'internal_line_remove': 'Retirer la ligne',
      'seam_add': 'Coudre',
      'seam_remove': 'Découdre',
      'seam_flip': 'Inverser la couture',
      'seam_set_gathered': 'Froncer la couture',
      'edge_lengthen': 'Rallonger',
      'edge_set_length': 'Fixer la longueur',
      'seam_take_in': 'Reprendre la couture',
      'seam_match_length': 'Égaliser les longueurs',
      'pin_add': 'Épingler',
      'pin_remove': 'Retirer l’épingle',
      'garment_set_fabric': 'Changer de tissu',
      'garment_arrange': 'Disposer les pièces',
      'garment_set_color': 'Changer la couleur',
      'garment_add': 'Ajouter un vêtement',
      'garment_rename': 'Renommer le vêtement',
      'garment_remove': 'Retirer le vêtement',
      'garment_set_layer': 'Changer de couche',
      'panel_set_garment': 'Changer de vêtement',
      'grading_set_sizes': 'Définir les tailles',
      'grading_use_chart': 'Utiliser un barème',
      'grading_clear': 'Retirer les tailles',
      'grading_show_size': 'Afficher la taille',
      'point_set_grade': 'Graduer le point',
      'template_apply': 'Ajouter un modèle',
      'pattern_import': 'Importer un patron',
      'batch': 'Plusieurs modifications',
      'constraint_add': 'Ajouter une contrainte',
      'constraint_remove': 'Retirer une contrainte',
      'constraint_set_value': 'Changer une cote',
      'point_set_smooth': 'Point lisse',
      'panel_unfold': 'Déplier la pièce',
      'guide_add': 'Ajouter une ligne de construction',
      'guide_move': 'Déplacer une ligne de construction',
      'guide_remove': 'Retirer une ligne de construction',
      'panel_create_ellipse': 'Ajouter une ellipse',
      'panel_create_sector': 'Ajouter une jupe cercle',
      'edge_set_arc': 'Faire un arc',
      'constraint_set_formula': 'Formule de cote',
      'variable_set': 'Modifier une variable',
      'variable_remove': 'Retirer une variable',
      'seam_set_ease': 'Régler l’embu',
      'dart_add': 'Ajouter une pince',
      'pleat_add': 'Ajouter un pli',
      'panel_add_hole': 'Découper un trou',
      'panel_remove_hole': 'Boucher un trou',
      'panel_boolean': 'Joindre ou découper',
      'other': '$step',
    });
    return '$_temp0';
  }

  @override
  String undoStep(String name) {
    return 'Annuler $name  ⌘Z';
  }

  @override
  String redoStep(String name) {
    return 'Rétablir $name  ⇧⌘Z';
  }

  @override
  String get nothingToUndo => 'Rien à annuler';

  @override
  String get nothingToRedo => 'Rien à rétablir';

  @override
  String get historyTip => 'Historique : revenir à n’importe quelle étape';

  @override
  String get historyTitle => 'Historique';

  @override
  String get historyStart => 'Ouverture';

  @override
  String get historyEmpty => 'Aucune modification pour l’instant.';

  @override
  String copied(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pièces copiées',
      one: '1 pièce copiée',
    );
    return '$_temp0, avec les coutures entre elles. ⌘V les colle, ici ou dans un autre projet.';
  }

  @override
  String pasted(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pièces collées',
      one: '1 pièce collée',
    );
    return '$_temp0.';
  }

  @override
  String get nothingToPaste =>
      'Rien à coller : copiez d’abord des pièces (⌘C).';

  @override
  String get selectToCopy => 'Sélectionnez les pièces à copier.';

  @override
  String get errCancelled => 'Arrêté : rien n’a été modifié.';

  @override
  String get busyStop => 'Arrêter';

  @override
  String get busyStep => 'En cours…';

  @override
  String busySizes(String size, int done, int total) {
    return 'Tracé de la taille $size ($done sur $total)';
  }

  @override
  String busySizesStart(int total) {
    return 'Tracé de chaque taille ($total)';
  }

  @override
  String get busyBodyFit => 'Ajustement du corps aux mesures…';

  @override
  String get busyDrape => 'Mise en place des pièces autour du corps…';

  @override
  String get busyExport => 'Export en cours…';

  @override
  String get busyImport => 'Lecture du patron…';

  @override
  String get busyOpening => 'Ouverture du projet';

  @override
  String get busyTemplate => 'Ajout du modèle';

  @override
  String get busySizesTitle => 'Réglage des tailles';

  @override
  String get busyBodyTitle => 'Modification du corps';

  @override
  String get busyDrapeTitle => 'Lancement du drapé';

  @override
  String get bodyFitting => 'Ajustement du corps…';

  @override
  String get toolShapes => 'Formes';

  @override
  String get toolEllipse => 'Ellipse';

  @override
  String get toolEllipseHint =>
      'Un cercle ou une ellipse aux dimensions saisies';

  @override
  String get toolSector => 'Jupe cercle';

  @override
  String get toolSectorHint =>
      'Des pièces en anneau d’après le tour de taille et la longueur de jupe';

  @override
  String get toolArc => 'Arc';

  @override
  String get toolArcHint => 'Cliquez une arête pour en faire un arc de cercle';

  @override
  String get toolGuide => 'Ligne de construction';

  @override
  String get toolGuideHint =>
      'Cliquez deux points : une ligne pour tracer, jamais coupée ni cousue';

  @override
  String get hintGuide =>
      'Ligne de construction : cliquez deux points · Échap annule · V pour arrêter';

  @override
  String get toolDimension => 'Cote';

  @override
  String get toolDimensionHint =>
      'Cliquez un bord, deux coins ou deux bords, puis là où va la cote ; tapez sa valeur ou une formule';

  @override
  String get hintDimension =>
      'Cote : cliquez un bord (sa longueur), un coin puis un autre (la distance) ou deux bords (l’angle) · V pour arrêter';

  @override
  String get hintDimensionPlace =>
      'Cote : cliquez là où elle va (entre deux coins, tirez-la au-dessus ou à côté pour une largeur ou une hauteur), puis tapez sa valeur · Échap annule';

  @override
  String get hintArc => 'Arc : cliquez une arête · V pour arrêter';

  @override
  String get toastGuideAdded => 'Ligne de construction ajoutée.';

  @override
  String get symmetricHalf => 'Demi-pièce';

  @override
  String get symmetricHalfTip =>
      'Tracez la moitié d’une pièce symétrique : l’arête de fermeture est l’axe, l’autre moitié suit';

  @override
  String get width => 'Largeur';

  @override
  String get height => 'Hauteur';

  @override
  String get sectorWaist => 'Tour de taille';

  @override
  String get sectorLength => 'Longueur de jupe';

  @override
  String get sectorFull => 'Complète';

  @override
  String get sectorHalf => 'Demi';

  @override
  String get sectorQuarter => 'Quart';

  @override
  String get sectorHint =>
      'Une jupe cercle complète se fait en deux demi-pièces, à coudre sur les côtés.';

  @override
  String get arcTitle => 'Arc';

  @override
  String get arcBulge => 'Flèche';

  @override
  String get arcHint =>
      'Distance entre le milieu de l’arc et la ligne droite ; négative, l’arc rentre vers l’intérieur.';

  @override
  String get constraints => 'Contraintes';

  @override
  String get constraintsEmpty =>
      'Choisissez un coin ou une arête sur le patron (⌘-clic pour un second) pour les contraindre.';

  @override
  String constraintsFreedom(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count degrés de liberté restants',
      one: '1 degré de liberté restant',
      zero: 'Entièrement contraint',
    );
    return '$_temp0';
  }

  @override
  String get constraintsRedundant =>
      'Certaines contraintes en répètent d’autres.';

  @override
  String get constraintBroken => 'Non respectée';

  @override
  String constraintKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'fixed': 'Fixe',
      'coincident': 'Joints',
      'horizontal': 'Horizontale',
      'vertical': 'Verticale',
      'parallel': 'Parallèles',
      'perpendicular': 'D’équerre',
      'tangent': 'Lisse',
      'equal_length': 'Même longueur',
      'on_edge': 'Sur l’arête',
      'distance': 'Distance',
      'width': 'Largeur',
      'height': 'Hauteur',
      'length': 'Longueur',
      'angle': 'Angle',
      'other': '$kind',
    });
    return '$_temp0';
  }

  @override
  String get constrain => 'Contraindre';

  @override
  String get removeConstraint => 'Retirer';

  @override
  String get pointSmooth => 'Lisse';

  @override
  String get pointCorner => 'Coin';

  @override
  String get pointRemove => 'Retirer le point';

  @override
  String get symmetricPiece => 'Pièce symétrique';

  @override
  String get unlinkHalves => 'Séparer les moitiés';

  @override
  String get unfold => 'Déplier ici';

  @override
  String get makeArc => 'Arc';

  @override
  String snapKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'point': 'Point',
      'midpoint': 'Milieu',
      'edge': 'Sur l’arête',
      'guide': 'Sur la ligne',
      'axis': 'Sur l’axe',
      'horizontal': 'Même hauteur',
      'vertical': 'Aligné',
      'other': '$kind',
    });
    return '$_temp0';
  }

  @override
  String get errDerived =>
      'Cette partie suit la moitié tracée ou la pièce d’origine : modifiez celle-ci.';

  @override
  String get errUnsolvable =>
      'Ces contraintes ne peuvent pas toutes tenir : rien n’a été changé.';

  @override
  String get errSymmetric => 'Pièce symétrique : séparez d’abord ses moitiés.';

  @override
  String get errNotStraight => 'Il faut une arête droite.';

  @override
  String get errDuplicate => 'Déjà contraint de cette façon.';

  @override
  String get formulaTip =>
      'Formule : calculer cette valeur d’après les mesures du corps';

  @override
  String get formulaTitle => 'Formule';

  @override
  String get formulaHelp =>
      'Mesures : height (stature), bust (poitrine), waist (taille), hips (hanches). Unités : mm, cm, in. + − × ÷, min, max. Vos variables aussi. Vide : la valeur reste telle quelle.';

  @override
  String get variables => 'Variables';

  @override
  String get variablesHint =>
      'Des noms pour les formules, comme ease = 2 cm. Les pièces à formules se redessinent pour un nouveau corps et dans chaque taille.';

  @override
  String get variable => 'Variable';

  @override
  String get variableName => 'Nom';

  @override
  String get variableAdd => 'Ajouter une variable';

  @override
  String get errGradedByFormula =>
      'Ce point est gradué par des formules : modifiez-les plutôt.';

  @override
  String errFormulaSyntax(String at) {
    return 'La formule est illisible au caractère $at.';
  }

  @override
  String errFormulaUnknown(String name) {
    return '« $name » n’est ni une mesure ni une variable.';
  }

  @override
  String errFormulaNoBody(String name) {
    return '« $name » demande les mesures du corps : saisissez-les d’abord.';
  }

  @override
  String errFormulaCycle(String name) {
    return '« $name » se calcule à partir de lui-même.';
  }

  @override
  String get errFormulaUnit =>
      'Les unités ne correspondent pas (une longueur prend une unité, comme 10 mm).';

  @override
  String get errFormulaDomain =>
      'La formule n’a pas de valeur (une division par zéro ?).';

  @override
  String errFormulaRange(String value) {
    return 'La formule donne $value : trop petit pour cette cote.';
  }

  @override
  String errFormulaBadName(String name) {
    return '« $name » ne peut pas nommer une variable : une lettre, puis des lettres, chiffres ou _.';
  }

  @override
  String get toolDetails => 'Détails';

  @override
  String get toolDart => 'Pince';

  @override
  String get toolDartHint =>
      'Cliquez une arête à l’endroit de la pince : tracée, pliée et cousue';

  @override
  String get toolPleat => 'Pli';

  @override
  String get toolPleatHint =>
      'Cliquez une arête là où commence le pli : plié et cousu';

  @override
  String get toolHole => 'Trou';

  @override
  String get toolHoleHint =>
      'Sélectionnez une pièce, puis cliquez les coins d’un trou à l’intérieur';

  @override
  String get hintDart =>
      'Pince : cliquez une arête à son emplacement · V pour arrêter';

  @override
  String get hintPleat =>
      'Pli : cliquez une arête là où il commence · V pour arrêter';

  @override
  String get hintHole =>
      'Trou : cliquez ses coins dans la pièce sélectionnée, Entrée pour le découper · Échap annule';

  @override
  String get toastHoleAdded => 'Trou découpé.';

  @override
  String get holeSelectFirst => 'Sélectionnez d’abord la pièce à trouer.';

  @override
  String get dartWidth => 'Largeur au bord';

  @override
  String get dartLength => 'Longueur';

  @override
  String get pleatDepth => 'Profondeur';

  @override
  String get pleatStitched => 'Piqué sur';

  @override
  String get pleatKnife => 'Plat';

  @override
  String get pleatBox => 'Creux';

  @override
  String get combine => 'Joindre ou découper';

  @override
  String get combineWith => 'Avec la pièce';

  @override
  String get join => 'Joindre';

  @override
  String get cutOut => 'Découper';

  @override
  String get ease => 'Embu';

  @override
  String get errOverlaps => 'Ce tronçon est déjà cousu.';

  @override
  String get errNotWholeEdges =>
      'Cela marche sur les coutures entre deux arêtes entières.';

  @override
  String get errApart => 'Le résultat ferait plusieurs pièces, ou rien.';

  @override
  String get errHasHoles => 'Cette pièce a des trous : bouchez-les d’abord.';

  @override
  String get errNotJoined => 'Ces arêtes ne se suivent pas sur une même pièce.';
}
