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
