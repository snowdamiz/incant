# p3a: Unicode-string commits: Arabic into the Arabic field, Tab to CJK (R1), CJK into the
# CJK field, then blur with CJK focused (R2).
$W $P raise && $W hold 0.6 &&
$W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W $P unicode " مرحبا" && $W hold 0.4 &&
$W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W $P unicode "日本語" && $W hold 0.4 &&
$W activate $PREV >/dev/null && $W hold 0.8
