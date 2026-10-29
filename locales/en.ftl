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
