# Crate Tab – English

## Language and formats

language-name = English
fmt-decimal-separator = .
fmt-thousands-separator = ,
fmt-money = { $sign }{ $currency }{ $amount }
fmt-datetime = %Y-%m-%d %H:%M
fmt-date = %Y-%m-%d

## General

app-name = Crate Tab
app-tagline = The drinks tab for your club – written in Rust 🦀
language-switch = Choose language
back = Back
cancel = Cancel
save = Save
saved = Saved.
upload = Upload
status-active = active
status-inactive = inactive
keypad-back = Delete last digit
keypad-clear = Clear input

## Kiosk – start page

user-search = Search name …
user-search-empty = Nobody found.
users-empty = No users yet. Go ahead and create yourself!
letters-all = All
register-button = New here?
pin-protected = protected by PIN
admin-link = Admin area

## Kiosk – registration

register-title = Sign up
register-name = Name
register-pin-hint = Optional: with a PIN (4–8 digits) nobody else can book on your name.
register-submit = Create

## Kiosk – PIN

pin-prompt = Please enter your PIN.
pin-label = PIN
pin-submit = Continue
pin-digits-only = The PIN consists of digits only.
pin-wrong = Wrong PIN.
pin-locked = Too many failed attempts. Please try again in { $minutes } minutes.
pin-new = New PIN
pin-repeat = Repeat PIN
pin-optional = PIN (optional)
pin-rules = The PIN consists of 4 to 8 digits.
pin-set-title = Set PIN
pin-change-title = Change PIN
pin-remove = Remove PIN
pin-remove-confirm = Really remove the PIN? Then anyone can book on your name.
pin-saved = PIN saved.
pin-removed = PIN removed.

## Kiosk – user page

hello-user = Hi { $name }!
done-button = Done
settings-button = Settings
balance-label = Balance
balance-min-hint = Your balance is negative. Allowed down to { $min }.
articles-title = Drinks & snacks
articles-empty = No articles yet.
scan-camera = Scan barcode
scan-hint = Tap an article or scan its barcode.
scan-select-user-first = Please tap your name first, then scan.
scan-unknown = Unknown barcode { $code }. Please tell an admin.
purchase-ok = { $article } booked ({ $price }). Cheers!
deposit-title = Top up balance
deposit-hint = Put the money in the cash box, then tap the amount.
deposit-custom = Other amount
deposit-custom-label = Amount (at most { $max })
deposit-submit = Top up
deposit-ok = { $amount } added. Thank you!
undo-button = Undo: { $what }
undo-ok = Booking reversed ({ $amount }).

## Kiosk – settings

settings-title = Settings
language-preference = Preferred language
language-preference-hint = After selecting your name, the kiosk switches to this language automatically.
language-device = Use the device language
language-saved = Language saved.

## Camera scanner

camera-title = Hold the barcode in front of the camera
camera-close = Close
camera-unavailable = No camera available or access denied.
camera-insecure = The camera only works via HTTPS.

## Error messages

error-internal = Something went wrong. Please try again.
error-network = No connection to the server. Please try again.
error-not-found = Not found.
error-name-empty = Please enter a name.
error-name-too-long = The name may be at most { $max } characters long.
error-name-taken = This name is already taken.
error-pin-invalid = The PIN must consist of 4 to 8 digits.
error-pin-mismatch = The two PINs do not match.
error-user-not-found = User not found or deactivated.
error-insufficient-balance = Not enough balance – please top up first. A balance down to { $min } is allowed.
error-article-unavailable = This article is not available.
error-amount-invalid = Please enter a valid amount.
error-amount-too-large = The amount is too high.
error-deposit-max = A single top-up may be at most { $max }.
error-nothing-to-undo = There is nothing left to undo.
error-reason-required = Please give a reason.
error-price-invalid = Please enter a valid price (e.g. 1.50).
error-barcode-invalid = Invalid barcode: { $code }
error-barcode-taken = Barcode { $code } already belongs to “{ $article }”.
error-image-type = Please upload a PNG, JPEG, WebP or GIF image.
error-image-size = The image is too large (at most { $kb } KB).
error-password-short = The password must be at least { $min } characters long.
error-password-mismatch = The two passwords do not match.
error-password-wrong = The current password is wrong.

## Admin – general

admin-title = Admin area
admin-nav-dashboard = Overview
admin-nav-articles = Articles
admin-nav-users = Users
admin-nav-transactions = Bookings
admin-nav-stats = Statistics
admin-nav-admins = Admins
admin-nav-kiosk = To the kiosk
admin-logout = Log out ({ $name })
admin-login-title = Admin login
admin-login-submit = Log in
admin-login-failed = Wrong username or password.
admin-username = Username
admin-password = Password
admin-recent-transactions = Recent bookings
admin-show-all = Show all
admin-reversed = Booking reversed.
admin-balance-mismatch =
    { $count ->
        [one] For one user the balance differs from the sum of their bookings:
       *[other] For { $count } users the balance differs from the sum of their bookings:
    }

## Admin – articles

article-new = New article
article-name = Name
article-name-in = Name ({ $language })
article-name-fallback = empty = German name
article-price = Price
article-sort = Sort order
article-status = Status
article-active = Article is active (shown at the kiosk)
article-barcodes = Barcodes
article-barcodes-hint = One barcode per line. Simply scan into the field with a USB scanner.
article-image = Image
article-image-none = No image yet.
article-image-upload = Upload image (PNG, JPEG, WebP or GIF, at most { $kb } KB)
article-image-delete = Remove image
article-image-delete-confirm = Really remove the image?
article-move-up = Move up
article-move-down = Move down

## Admin – users

user-new = Create user
user-since = Created
user-data = User data
user-active = User is active
user-active-hint = Deactivated users no longer appear at the kiosk. Their bookings are kept.
user-has-pin = The user has a PIN.
user-no-pin = The user has no PIN.
pin-reset = Reset PIN
pin-reset-label = New PIN (leave empty to remove the PIN)
pin-reset-confirm = Really reset this user's PIN?
correction-title = Correct balance
correction-hint = A positive amount increases the balance, a negative one decreases it. The reason is shown in the booking list.
correction-amount = Amount
correction-reason = Reason (required)
correction-submit = Book correction

## Admin – bookings

tx-time = Time
tx-user = User
tx-kind = Type
tx-article = Article
tx-amount = Amount
tx-note = Note
tx-sum = Total
tx-empty = No bookings found.
tx-reverse = Reverse
tx-reverse-confirm = Really reverse booking #{ $id }? A counter-booking will be created.
tx-reverses = reversal of #{ $id }
tx-reversed-by = reversed by #{ $id }
tx-count =
    { $count ->
        [one] 1 booking
       *[other] { $count } bookings
    }
tx-kind-purchase = Purchase
tx-kind-deposit = Top-up
tx-kind-correction = Correction
tx-kind-reversal = Reversal
filter-all = all
filter-from = from
filter-to = to
filter-apply = Filter
filter-reset = Reset
export-csv = CSV export
page-prev = Previous
page-next = Next
page-current = Page { $page }

## CSV export (column headers)

csv-id = No
csv-time = Time
csv-user = User
csv-kind = Type
csv-article = Article
csv-amount = Amount
csv-amount-cents = Amount (cents)
csv-note = Note
csv-admin = Admin
csv-reverses = Reversal of

## Admin – statistics

stats-total-balance = Sum of all balances
stats-credit = of which credit
stats-debt = of which debt
stats-users = Users
stats-month-revenue = Revenue this month
stats-month-deposits = Top-ups this month
stats-year = Year
stats-year-revenue = Revenue { $year }
stats-year-deposits = Top-ups { $year }
stats-by-article = Revenue per article { $year }
stats-by-month = Per month
stats-quantity = Quantity
stats-revenue = Revenue
stats-deposits = Top-ups

## Admin – accounts

admin-new = Create new admin
admin-me = you
admin-activate = Activate
admin-deactivate = Deactivate
admin-change-password = Change your password
admin-password-current = Current password
admin-password-new = New password
admin-password-repeat = Repeat new password

## Admin – import from fnordcredit

admin-nav-import = Import
import-title = Import from fnordcredit
import-intro = Takes over users, balances, PINs, products and the complete booking history from the old fnordcredit. After uploading, a dry run is done first. Nothing is saved until you confirm.
import-howto = How to create the export is described in the guide:
import-not-empty = The database is not empty. Importing is only possible if the existing data is replaced. Currently present:
import-file = Export file
import-file-hint = fnordcredit-export.tar.gz – or select users.json, transactions.json and products.json together.
import-upload = Upload and check
import-no-file = Please select a file.
import-error = Import not possible: { $error }
import-expired = The upload has expired or was already used. Please upload the file again.
import-dry-run-title = Dry run – nothing has been saved yet. Please check the report and then confirm.
import-done-title = Import completed.
import-files = Read
import-users = Users
import-articles = Articles
import-transactions = Bookings
import-barcodes =
    { $count ->
        [one] 1 barcode
       *[other] { $count } barcodes
    }
import-purchases =
    { $count ->
        [one] 1 purchase
       *[other] { $count } purchases
    }
import-deposits =
    { $count ->
        [one] 1 top-up
       *[other] { $count } top-ups
    }
import-images =
    { $count ->
        [one] 1 image
       *[other] { $count } images
    }
import-hint-images = Images not taken over
import-hint-images-text = Missing images: repeat the export with the current version of scripts/fnordcredit-export.js. All images can also be uploaded later for the article in the admin area.
import-image-missing = image file missing in the export
import-image-unsupported = format not supported (only PNG, JPEG, GIF, WebP)
import-image-too-large = image is too large
import-pins = PINs taken over
import-pins-hint = converted to argon2 on first login
import-total-old = Sum of all balances in fnordcredit
import-total-new = Sum of all balances in Crate Tab
import-total-orphans = of which { $amount } for users without an old balance
import-total-ok = The sums of all balances match.
import-total-mismatch = The sums of all balances differ – please check!
import-hints = Notes
import-hint-reconcile = Balancing bookings
import-hint-reconcile-text = For these users the sum of bookings does not match the balance in fnordcredit (e.g. users from before the booking history). A correction booking balances the difference.
import-col-balance = Balance in fnordcredit
import-col-sum = Sum of bookings
import-col-difference = Balancing
import-hint-renamed = Renamed (duplicate or too long name)
import-hint-orphans = Bookings without a user – created as deactivated users
import-hint-pins-dropped = PIN in unknown format, not taken over
import-hint-tokens = Had a token (dropped)
import-hint-debt = Had individual debt limits (dropped, the global minimum balance applies)
import-debt-none = no debt allowed
import-debt-limit = limit { $amount }
import-hint-unknown-products = Products from bookings that no longer exist (kept as name only)
import-hint-barcodes = Barcodes skipped
import-barcode-invalid = invalid
import-barcode-taken = already belongs to { $article }
import-hint-unreadable = Unreadable records
import-hint-zero =
    { $count ->
        [one] 1 booking without amount skipped.
       *[other] { $count } bookings without amount skipped.
    }
import-hint-no-time =
    { $count ->
        [one] 1 booking without timestamp skipped.
       *[other] { $count } bookings without timestamp skipped.
    }
import-hint-chain =
    { $count ->
        [one] 1 booking in fnordcredit did not match the previous balance (sign of earlier changes made directly in the database).
       *[other] { $count } bookings in fnordcredit did not match the previous balance (sign of earlier changes made directly in the database).
    }
import-replace = Replace existing data
import-replace-hint = Users, articles and bookings are deleted and replaced by the import. Admin accounts are kept. A backup is created automatically beforehand. The dry run does not delete anything yet.
import-replace-will-delete = The import will delete:
import-replace-deleted = Deleted and replaced:
import-replace-admins-kept = Admin accounts are kept.
import-replace-ack = I understand that the existing users, articles and bookings will be deleted.
import-replace-backup-hint = Before replacing, the current database is backed up to the “backups” folder next to the database file.
import-replace-confirm = Replace data and import
import-replace-confirm-question = Really delete the existing data now and replace it with the import?
import-replace-ack-missing = Please confirm that the existing data will be deleted.
import-backup-created = Backup of the previous database:
import-backup-failed = The backup before replacing failed, nothing was changed: { $error }
import-confirm = Import now
import-confirm-question = Take over the data now?
import-confirm-hint = The upload stays valid for 30 minutes. After that, please upload it again.
import-to-users = To the users
