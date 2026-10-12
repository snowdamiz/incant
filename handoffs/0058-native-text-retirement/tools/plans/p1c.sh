# p1c: Tab to Name, capture all three fields, then close through the AX close button with
# Name focused (retirement R9 at exit).
$W $P raise && $W hold 0.6 &&
$W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W hold 0.3 &&
$C p1-03-after-retirements &&
$W $P close >/dev/null
