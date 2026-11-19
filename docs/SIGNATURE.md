# Signer l'installateur gratuitement (SignPath Foundation)

Sans signature de code, Windows affiche « Windows a protégé votre ordinateur » (SmartScreen)
au premier lancement de l'installateur. Ça fait peur aux joueurs, et ça peut freiner les téléchargements.

Un certificat payant coûte environ 100 à 300 € par an. Comme Meiki Tracker est **open source**, il existe une
option gratuite : **SignPath Foundation**. Elle signe gratuitement les projets open source qui
respectent ses conditions.

> À ne pas confondre avec la **signature des mises à jour**, déjà en place (clé dans
> `%USERPROFILE%\.tauri\`). Celle-ci garantit que l'appli n'installe que tes mises à jour, mais elle
> ne retire pas l'avertissement SmartScreen.

## Conditions principales (à vérifier sur leur site, elles peuvent évoluer)

- Le dépôt est public, sous une licence open source reconnue (MIT : ✅).
- Le projet est réellement utilisé et maintenu, et a une page claire (README : ✅).
- L'installateur est compilé par un **service de compilation automatique** (GitHub Actions),
  et pas sur ton PC : SignPath doit pouvoir vérifier que le fichier signé vient bien du code publié.
- Les mentions demandées sont présentes (une ligne dans le README / la page de téléchargement).

## Étapes

1. Publie le dépôt et une première version sur GitHub.
2. Va sur **https://signpath.org**, rubrique pour les projets open source, et remplis la demande :
   - nom du projet ;
   - lien GitHub ;
   - courte description (« Suivi du temps de jeu pour Windows, 100 % local ») ;
   - licence MIT.
3. Attends leur réponse. Ça peut prendre plusieurs jours ou semaines.
4. S'ils acceptent, ils te guident pour ajouter une étape de signature dans GitHub Actions.
   **Reviens me voir à ce moment-là** : il faudra créer un workflow de compilation
   (`.github/workflows/release.yml`) qui compile, signe via SignPath, puis publie la Release.

## En attendant

L'installateur fonctionne très bien sans signature. Le README explique aux joueurs comment passer
l'avertissement : « Informations complémentaires → Exécuter quand même ». SmartScreen se calme aussi
tout seul quand un fichier a été téléchargé par assez de monde.
