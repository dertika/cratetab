# Crate Tab – Français

## Langue et formats

language-name = Français
fmt-decimal-separator = ,
# Espace fine insécable comme séparateur de milliers
fmt-thousands-separator = {" "}
fmt-money = { $sign }{ $amount }{" "}{ $currency }
fmt-datetime = %d/%m/%Y %H:%M
fmt-date = %d/%m/%Y

## Général

app-name = Crate Tab
app-tagline = La caisse à boissons de votre association – écrite en Rust 🦀
language-switch = Choisir la langue
back = Retour
cancel = Annuler
save = Enregistrer
saved = Enregistré.
upload = Envoyer
status-active = actif
status-inactive = inactif
keypad-back = Effacer le dernier chiffre
keypad-clear = Effacer la saisie

## Kiosque – accueil

user-search = Chercher un nom …
user-search-empty = Personne trouvé.
users-empty = Pas encore d’utilisateurs. Inscris-toi tout de suite !
letters-all = Tous
register-button = Nouveau ici ?
pin-protected = protégé par un code PIN
admin-link = Espace admin

## Kiosque – inscription

register-title = S’inscrire
register-name = Nom
register-pin-hint = Facultatif : avec un code PIN (4 à 8 chiffres), personne d’autre ne peut acheter à ton nom.
register-submit = Créer

## Kiosque – code PIN

pin-prompt = Saisis ton code PIN.
pin-label = Code PIN
pin-submit = Continuer
pin-digits-only = Le code PIN ne contient que des chiffres.
pin-wrong = Code PIN incorrect.
pin-locked = Trop d’essais. Réessaie dans { $minutes } minutes.
pin-new = Nouveau code PIN
pin-repeat = Répéter le code PIN
pin-optional = Code PIN (facultatif)
pin-rules = Le code PIN comporte 4 à 8 chiffres.
pin-set-title = Définir un code PIN
pin-change-title = Changer le code PIN
pin-remove = Supprimer le code PIN
pin-remove-confirm = Vraiment supprimer le code PIN ? Tout le monde pourra alors acheter à ton nom.
pin-saved = Code PIN enregistré.
pin-removed = Code PIN supprimé.

## Kiosque – page utilisateur

hello-user = Salut { $name } !
done-button = Terminé
settings-button = Réglages
balance-label = Solde
balance-min-hint = Ton solde est négatif. Autorisé jusqu’à { $min }.
articles-title = Boissons & snacks
articles-empty = Pas encore d’articles.
scan-camera = Scanner un code-barres
scan-hint = Touche un article ou scanne son code-barres.
scan-select-user-first = Touche d’abord ton nom, puis scanne.
scan-unknown = Code-barres inconnu { $code }. Préviens un·e admin, s’il te plaît.
purchase-ok = { $article } enregistré ({ $price }). Santé !
deposit-title = Recharger le solde
deposit-hint = Mets l’argent dans la caisse, puis touche le montant.
deposit-custom = Autre montant
deposit-custom-label = Montant (au maximum { $max })
deposit-submit = Recharger
deposit-ok = { $amount } rechargé. Merci !
undo-button = Annuler : { $what }
undo-ok = Opération annulée ({ $amount }).

## Kiosque – réglages

settings-title = Réglages
language-preference = Langue préférée
language-preference-hint = Après avoir choisi ton nom, le kiosque passe automatiquement à cette langue.
language-device = Utiliser la langue de l’appareil
language-saved = Langue enregistrée.

## Scanner caméra

camera-title = Présente le code-barres devant la caméra
camera-close = Fermer
camera-unavailable = Aucune caméra disponible ou accès refusé.
camera-insecure = La caméra ne fonctionne qu’en HTTPS.

## Messages d’erreur

error-internal = Une erreur s’est produite. Réessaie, s’il te plaît.
error-network = Pas de connexion au serveur. Réessaie, s’il te plaît.
error-not-found = Introuvable.
error-name-empty = Saisis un nom, s’il te plaît.
error-name-too-long = Le nom peut comporter au maximum { $max } caractères.
error-name-taken = Ce nom est déjà pris.
error-pin-invalid = Le code PIN doit comporter 4 à 8 chiffres.
error-pin-mismatch = Les deux codes PIN ne correspondent pas.
error-user-not-found = Utilisateur introuvable ou désactivé.
error-insufficient-balance = Solde insuffisant – recharge d’abord, s’il te plaît. Un solde jusqu’à { $min } est autorisé.
error-article-unavailable = Cet article n’est pas disponible.
error-amount-invalid = Saisis un montant valide, s’il te plaît.
error-amount-too-large = Le montant est trop élevé.
error-balance-max = Avec cette recharge, le solde dépasserait le maximum de { $max }.
error-deposit-max = Une recharge ne peut pas dépasser { $max }.
error-nothing-to-undo = Il n’y a plus rien à annuler.
error-reason-required = Indique un motif, s’il te plaît.
error-price-invalid = Saisis un prix valide (par ex. 1,50).
error-barcode-invalid = Code-barres invalide : { $code }
error-barcode-taken = Le code-barres { $code } appartient déjà à « { $article } ».
error-image-type = Envoie une image au format PNG, JPEG, WebP ou GIF.
error-image-size = L’image est trop grande (au maximum { $kb } Ko).
error-password-short = Le mot de passe doit comporter au moins { $min } caractères.
error-password-mismatch = Les deux mots de passe ne correspondent pas.
error-password-wrong = Le mot de passe actuel est incorrect.

## Admin – général

admin-title = Espace admin
admin-nav-dashboard = Vue d’ensemble
admin-nav-articles = Articles
admin-nav-users = Utilisateurs
admin-nav-transactions = Opérations
admin-nav-stats = Statistiques
admin-nav-admins = Admins
admin-nav-kiosk = Vers le kiosque
admin-logout = Se déconnecter ({ $name })
admin-login-title = Connexion admin
admin-login-submit = Se connecter
admin-login-failed = Nom d’utilisateur ou mot de passe incorrect.
admin-username = Nom d’utilisateur
admin-password = Mot de passe
admin-recent-transactions = Dernières opérations
admin-show-all = Tout afficher
admin-reversed = Opération annulée.
admin-balance-mismatch =
    { $count ->
        [one] Pour un utilisateur, le solde diffère de la somme de ses opérations :
       *[other] Pour { $count } utilisateurs, le solde diffère de la somme de leurs opérations :
    }

## Admin – articles

article-new = Nouvel article
article-name = Nom
article-name-in = Nom ({ $language })
article-name-fallback = vide = nom allemand
article-price = Prix
article-sort = Ordre
article-status = Statut
article-active = L’article est actif (affiché au kiosque)
article-barcodes = Codes-barres
article-barcodes-hint = Un code-barres par ligne. Il suffit de scanner dans le champ avec un lecteur USB.
article-image = Image
article-image-none = Pas encore d’image.
article-image-upload = Envoyer une image (PNG, JPEG, WebP ou GIF, au maximum { $kb } Ko)
article-image-delete = Supprimer l’image
article-image-delete-confirm = Vraiment supprimer l’image ?
article-move-up = Monter
article-move-down = Descendre

## Admin – utilisateurs

user-new = Créer un utilisateur
user-since = Créé le
user-data = Données
user-active = L’utilisateur est actif
user-active-hint = Les utilisateurs désactivés n’apparaissent plus au kiosque. Leurs opérations sont conservées.
user-has-pin = L’utilisateur a un code PIN.
user-no-pin = L’utilisateur n’a pas de code PIN.
pin-reset = Réinitialiser le code PIN
pin-reset-label = Nouveau code PIN (laisser vide pour le supprimer)
pin-reset-confirm = Vraiment réinitialiser le code PIN de cet utilisateur ?
correction-title = Corriger le solde
correction-hint = Un montant positif augmente le solde, un montant négatif le diminue. Le motif apparaît dans la liste des opérations.
correction-amount = Montant
correction-reason = Motif (obligatoire)
correction-submit = Enregistrer la correction

## Admin – opérations

tx-time = Heure
tx-user = Utilisateur
tx-kind = Type
tx-article = Article
tx-amount = Montant
tx-note = Note
tx-sum = Total
tx-empty = Aucune opération trouvée.
tx-reverse = Annuler
tx-reverse-confirm = Vraiment annuler l’opération n° { $id } ? Une contre-opération sera créée.
tx-reverses = annulation du n° { $id }
tx-reversed-by = annulée par le n° { $id }
tx-count =
    { $count ->
        [one] 1 opération
       *[other] { $count } opérations
    }
tx-kind-purchase = Achat
tx-kind-deposit = Recharge
tx-kind-correction = Correction
tx-kind-reversal = Annulation
filter-all = tous
filter-from = du
filter-to = au
filter-apply = Filtrer
filter-reset = Réinitialiser
export-csv = Export CSV
page-prev = Précédent
page-next = Suivant
page-current = Page { $page }

## Export CSV (en-têtes de colonnes)

csv-id = N°
csv-time = Heure
csv-user = Utilisateur
csv-kind = Type
csv-article = Article
csv-amount = Montant
csv-amount-cents = Montant (centimes)
csv-note = Note
csv-admin = Admin
csv-reverses = Annulation de

## Admin – statistiques

stats-total-balance = Somme de tous les soldes
stats-credit = dont avoirs
stats-debt = dont dettes
stats-users = Utilisateurs
stats-month-revenue = Chiffre d’affaires ce mois-ci
stats-month-deposits = Recharges ce mois-ci
stats-year = Année
stats-year-revenue = Chiffre d’affaires { $year }
stats-year-deposits = Recharges { $year }
stats-by-article = Chiffre d’affaires par article { $year }
stats-by-month = Par mois
stats-quantity = Quantité
stats-revenue = Chiffre d’affaires
stats-deposits = Recharges

## Admin – comptes

admin-new = Créer un nouvel admin
admin-me = toi
admin-activate = Activer
admin-deactivate = Désactiver
admin-change-password = Changer ton mot de passe
admin-password-current = Mot de passe actuel
admin-password-new = Nouveau mot de passe
admin-password-repeat = Répéter le nouveau mot de passe

## Admin – import depuis fnordcredit

admin-nav-import = Import
import-title = Import depuis fnordcredit
import-intro = Reprend les utilisateurs, soldes, codes PIN, produits et tout l’historique des opérations de l’ancien fnordcredit. Après l’envoi, un essai à blanc est d’abord effectué. Rien n’est enregistré avant ta confirmation.
import-howto = La création de l’export est décrite dans le guide :
import-not-empty = La base de données n’est pas vide. L’import n’est possible qu’en remplaçant les données existantes. Actuellement présents :
import-file = Fichier d’export
import-file-hint = fnordcredit-export.tar.gz – ou sélectionner ensemble users.json, transactions.json et products.json.
import-upload = Envoyer et vérifier
import-no-file = Choisis un fichier, s’il te plaît.
import-error = Import impossible : { $error }
import-expired = L’envoi a expiré ou a déjà été utilisé. Envoie à nouveau le fichier, s’il te plaît.
import-dry-run-title = Essai à blanc – rien n’a encore été enregistré. Vérifie le rapport puis confirme.
import-done-title = Import terminé.
import-files = Lu
import-users = Utilisateurs
import-articles = Articles
import-transactions = Opérations
import-barcodes =
    { $count ->
        [one] 1 code-barres
       *[other] { $count } codes-barres
    }
import-purchases =
    { $count ->
        [one] 1 achat
       *[other] { $count } achats
    }
import-deposits =
    { $count ->
        [one] 1 recharge
       *[other] { $count } recharges
    }
import-images =
    { $count ->
        [one] 1 image
       *[other] { $count } images
    }
import-hint-images = Images non reprises
import-hint-images-text = Images manquantes : refaire l’export avec la version actuelle de scripts/fnordcredit-export.js. Toutes les images peuvent aussi être envoyées plus tard pour l’article dans l’espace admin.
import-image-missing = fichier image absent de l’export
import-image-unsupported = format non pris en charge (uniquement PNG, JPEG, GIF, WebP)
import-image-too-large = image trop grande
import-pins = Codes PIN repris
import-pins-hint = convertis en argon2 lors de la première connexion
import-total-old = Somme de tous les soldes dans fnordcredit
import-total-new = Somme de tous les soldes dans Crate Tab
import-total-orphans = dont { $amount } pour des utilisateurs sans ancien solde
import-total-ok = Les sommes de tous les soldes concordent.
import-total-mismatch = Les sommes de tous les soldes diffèrent – vérifie, s’il te plaît !
import-hints = Remarques
import-hint-reconcile = Opérations d’ajustement
import-hint-reconcile-text = Pour ces utilisateurs, la somme des opérations ne correspond pas au solde dans fnordcredit (p. ex. utilisateurs antérieurs à l’historique). Une correction compense la différence.
import-col-balance = Solde dans fnordcredit
import-col-sum = Somme des opérations
import-col-difference = Ajustement
import-hint-renamed = Renommés (nom en double ou trop long)
import-hint-orphans = Opérations sans utilisateur – créés comme utilisateurs désactivés
import-hint-pins-dropped = Code PIN dans un format inconnu, non repris
import-hint-tokens = Avaient un jeton (supprimé)
import-hint-debt = Avaient des limites de dette individuelles (supprimées, le solde minimum global s’applique)
import-debt-none = aucune dette autorisée
import-debt-limit = limite { $amount }
import-hint-unknown-products = Produits d’opérations qui n’existent plus (repris uniquement par leur nom)
import-hint-barcodes = Codes-barres ignorés
import-barcode-invalid = invalide
import-barcode-taken = appartient déjà à { $article }
import-hint-unreadable = Enregistrements illisibles
import-hint-zero =
    { $count ->
        [one] 1 opération sans montant ignorée.
       *[other] { $count } opérations sans montant ignorées.
    }
import-hint-no-time =
    { $count ->
        [one] 1 opération sans horodatage ignorée.
       *[other] { $count } opérations sans horodatage ignorées.
    }
import-hint-chain =
    { $count ->
        [one] 1 opération dans fnordcredit ne correspondait pas au solde précédent (signe de modifications faites directement dans la base de données).
       *[other] { $count } opérations dans fnordcredit ne correspondaient pas au solde précédent (signe de modifications faites directement dans la base de données).
    }
import-replace = Remplacer les données existantes
import-replace-hint = Les utilisateurs, articles et opérations sont supprimés et remplacés par l’import. Les comptes admin sont conservés. Une sauvegarde est créée automatiquement avant. L’essai à blanc ne supprime encore rien.
import-replace-will-delete = L’import supprimera :
import-replace-deleted = Supprimés et remplacés :
import-replace-admins-kept = Les comptes admin sont conservés.
import-replace-ack = J’ai compris que les utilisateurs, articles et opérations existants seront supprimés.
import-replace-backup-hint = Avant le remplacement, la base de données actuelle est sauvegardée dans le dossier « backups » à côté du fichier de base de données.
import-replace-confirm = Remplacer les données et importer
import-replace-confirm-question = Vraiment supprimer maintenant les données existantes et les remplacer par l’import ?
import-replace-ack-missing = Confirme que les données existantes seront supprimées, s’il te plaît.
import-backup-created = Sauvegarde de l’ancienne base de données :
import-backup-failed = La sauvegarde avant le remplacement a échoué, rien n’a été modifié : { $error }
import-confirm = Importer maintenant
import-confirm-question = Reprendre les données maintenant ?
import-confirm-hint = L’envoi reste valable 30 minutes. Ensuite, envoie-le à nouveau.
import-to-users = Vers les utilisateurs

## Admin – réglages

admin-nav-settings = Réglages
settings-page-title = Réglages
settings-intro = Les modifications s’appliquent immédiatement. Les valeurs sont enregistrées dans la base de données et font donc partie de chaque sauvegarde.
settings-section-general = Général
settings-section-balance = Solde et recharges
settings-section-kiosk = Kiosque
settings-club-name = Nom de l’association
settings-default-language = Langue par défaut
settings-self-registration = Les nouveaux utilisateurs peuvent s’inscrire eux-mêmes au kiosque
settings-min-balance = Solde minimum
settings-min-balance-hint = Solde le plus bas autorisé, p. ex. -10. Avec 0, aucune dette n’est possible.
settings-max-balance = Solde maximum
settings-max-balance-hint = Solde le plus élevé après une recharge au kiosque. Laisser vide pour aucune limite. Les corrections dans l’espace admin ne sont pas concernées.
settings-max-deposit = Montant maximum par recharge
settings-deposit-presets = Montants de recharge rapide
settings-deposit-presets-hint = Séparés par des espaces, p. ex. « 5 10 20 ». Au maximum 8 montants, chacun jusqu’au montant maximum par recharge.
settings-idle-timeout = Inactivité avant le retour à l’accueil (secondes)
settings-undo-window = Délai pour « annuler » (secondes)
settings-undo-window-hint = 0 désactive « annuler » au kiosque.
settings-source-stored = enregistré dans l’espace admin
settings-source-config = depuis la configuration ({ $variable })
settings-reset = Réinitialiser tous les réglages à la configuration
settings-reset-confirm = Supprimer toutes les valeurs enregistrées ici ? Ensuite, les valeurs de la configuration s’appliquent à nouveau.
settings-file-hint = Le chemin de la base de données, le port, le fuseau horaire et le mot de passe initial du premier admin restent dans la configuration (/etc/cratetab/cratetab.env ou docker-compose.yml).
settings-error-club-name = Saisis un nom d’association, s’il te plaît (80 caractères au maximum).
settings-error-min-balance = Le solde minimum ne doit pas être positif (p. ex. -10 ou 0).
settings-error-max-balance = Le solde maximum doit être positif et supérieur au solde minimum.
settings-error-max-deposit = Le montant maximum par recharge doit être positif.
settings-error-presets = Les montants de recharge rapide doivent être positifs et ne pas dépasser le montant maximum par recharge (8 au maximum).
settings-error-idle-timeout = L’inactivité doit être comprise entre 10 et 3600 secondes.
settings-error-undo-window = Le délai pour « annuler » doit être compris entre 0 et 3600 secondes.
settings-error-language = Cette langue n’existe pas.
