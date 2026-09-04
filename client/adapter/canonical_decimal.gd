class_name CanonicalDecimal
extends RefCounted

## Arithmetic and ordering for the wire's canonical decimal strings.
##
## Identifiers, counters, logical rounds, and wide signed quantities cross the
## wire as canonical decimal strings and [b]never pass through a float[/b]
## ([url]../../docs/client-architecture.md[/url]): a `double` silently loses
## precision at 2^53, and the shared fixture corpus carries exactly those
## boundary values as proof. Anything that has to compare or step one of those
## values therefore does it on the digits.
##
## Three copies of that logic had grown up independently — one in the adapter,
## one in the HUD, one in the interaction director — which is three chances for
## the same fact to be got subtly differently. This is the one owner.
##
## [b]Canonical[/b] means what the codec already enforced before any of this is
## reached: ASCII digits only, no sign, no leading zero except the single
## digit `0`. Every function here relies on that and on nothing else, which is
## why length is a valid first comparison.

## The largest difference [method bounded_difference] will report. A gap wider than
## this is not a number a presentation surface has any use for — it is "further
## away than anything worth drawing" — and reporting it as a saturated count
## keeps the result an ordinary `int` no matter how wide the operands were.
const MAXIMUM_REPORTED_DIFFERENCE: int = 999999


## True when `value` is a canonical decimal this class can reason about.
static func is_canonical(value: String) -> bool:
	if value.is_empty() or not value.is_valid_int():
		return false
	if value.begins_with("-") or value.begins_with("+"):
		return false
	return value == "0" or not value.begins_with("0")


static func less(left: String, right: String) -> bool:
	if left.length() != right.length():
		return left.length() < right.length()
	return left < right


static func at_least(left: String, right: String) -> bool:
	if right.is_empty():
		return false
	if left.length() != right.length():
		return left.length() > right.length()
	return left >= right


## `later - earlier` in whole rounds, floored at zero and saturated at
## [constant MAXIMUM_REPORTED_DIFFERENCE].
##
## Zero is the honest answer for `later <= earlier`, which on the pulse's own
## fields is the ordinary case rather than a fault: an actor ready at round 5
## while the world stands at round 9 is simply ready, and has been for four
## rounds.
static func bounded_difference(earlier: String, later: String) -> int:
	if not is_canonical(earlier) or not is_canonical(later):
		return 0
	if not less(earlier, later):
		return 0
	var digits: String = _subtract(later, earlier)
	if digits.length() > str(MAXIMUM_REPORTED_DIFFERENCE).length():
		return MAXIMUM_REPORTED_DIFFERENCE
	return mini(MAXIMUM_REPORTED_DIFFERENCE, int(digits))


## `value + 1`, on the digits, with no width limit.
static func increment(value: String) -> String:
	var digits: PackedByteArray = value.to_ascii_buffer()
	var carry: int = 1
	for index: int in range(digits.size() - 1, -1, -1):
		var next: int = digits[index] - 0x30 + carry
		digits[index] = 0x30 + next % 10
		carry = next / 10
		if carry == 0:
			break
	if carry == 1:
		digits.insert(0, 0x31)
	return digits.get_string_from_ascii()


## `left - right` on the digits, for `left >= right`, returned canonical.
static func _subtract(left: String, right: String) -> String:
	var minuend: PackedByteArray = left.to_ascii_buffer()
	var subtrahend: PackedByteArray = right.to_ascii_buffer()
	var borrow: int = 0
	var offset: int = minuend.size() - subtrahend.size()
	for index: int in range(minuend.size() - 1, -1, -1):
		var taken: int = 0 if index - offset < 0 else subtrahend[index - offset] - 0x30
		var next: int = minuend[index] - 0x30 - taken - borrow
		borrow = 0
		if next < 0:
			next += 10
			borrow = 1
		minuend[index] = 0x30 + next
	var result: String = minuend.get_string_from_ascii()
	while result.length() > 1 and result.begins_with("0"):
		result = result.substr(1)
	return result
