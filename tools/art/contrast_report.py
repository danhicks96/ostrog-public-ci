#!/usr/bin/env python3
"""Are faces readable? Prints the contrast between each face feature and the skin beside it.

Eyes are drawn in the fixed OUTLINE colour and the mouth/cheeks in the tone's own shadow step,
so the question is whether those still read against every skin tone the game can choose. A number
rather than a glance: this is the gate for "faces readable at 2x" (see the plan and art bible).

    python3 tools/art/contrast_report.py

Exits 1 if any feature falls under MIN_RATIO.
"""

import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import pal
import people

MIN_RATIO = 2.5


def rgb_to_hex(c):
    return "#%02x%02x%02x" % (int(c[0]), int(c[1]), int(c[2]))


def luminance(rgb):
    def chan(v):
        v = v / 255.0
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4

    r, g, b = (chan(float(x)) for x in rgb[:3])
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = luminance(a), luminance(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


def retint(frame, tone):
    """Paint a source-coloured layer in one skin tone, as the renderer does at runtime."""
    srcs = [tuple(int(v) for v in c[:3]) for c in people.SWAP_SOURCES["skin"]]
    dsts = [tuple(int(v) for v in c[:3]) for c in tone]
    out = frame.copy()
    for src, dst in zip(srcs, dsts):
        for y in range(out.shape[0]):
            for x in range(out.shape[1]):
                p = out[y, x]
                if p[3] and tuple(int(v) for v in p[:3]) == src:
                    out[y, x][:3] = dst
    return out


def skin_fill(img, head):
    """The colour most of the face actually is: features are judged against this."""
    hx0, hy0, hx1, hy1 = head
    counts = {}
    for y in range(hy0, hy1 + 1):
        for x in range(hx0, hx1 + 1):
            p = img[y, x]
            if p[3] == 0:
                continue
            c = tuple(int(v) for v in p[:3])
            counts[c] = counts.get(c, 0) + 1
    if not counts:
        return None
    return max(counts.items(), key=lambda kv: kv[1])[0]


def feature_contrast(img, head, x, y, fill):
    """Contrast of one feature pixel against the skin that surrounds the face."""
    px = tuple(int(v) for v in img[y, x][:3])
    if px == fill:
        return None  # nothing was drawn here
    return ratio(px, fill)


def main():
    worst = 99.0
    failures = []
    print("build       age      tone     eyes  mouth   (contrast vs the skin beside it)")
    for build in people.BUILDS:
        for age in people.AGES:
            for tone_name, tone in people.SKIN_TONES.items():
                img = people.make_frame("body", build, age, "S", "idle")
                frame = np.array(img).copy()
                # Layers ship in source colours; faces are only ever seen after the swap.
                frame = retint(frame, tone)
                s = people.skeleton(build, age, "S", "idle")
                head = s.head
                # Positions from the generator itself, so the gate follows the face as drawn.
                ey = people.face_eye_row(s)
                adult = age == "adult"
                lx, rx = people.face_eye_cols(s)
                pupil_dy, pupil_dx = (1, 1) if adult else (0, 0)
                left_eye = (lx + pupil_dx, ey + pupil_dy)
                right_eye = (rx + pupil_dx, ey + pupil_dy)
                left_lid = (lx, ey + 2)
                right_lid = (rx, ey + 2)
                my = ey + (5 if adult else 4)
                mouth = (s.cx, my)
                chin = (s.cx, my + 1)
                fill = skin_fill(frame, head)
                scores = {}
                for label, pos in (("eyes", left_eye), ("eyes_r", right_eye), ("mouth", mouth),
                                   ("lid", left_lid), ("lid_r", right_lid), ("chin", chin)):
                    got = feature_contrast(frame, head, pos[0], pos[1], fill)
                    if got is None:
                        continue
                    scores[label] = got
                # An eye reads if its pupil does, OR if the lit lid under it does.
                left = max(scores.get("eyes", 0.0), scores.get("lid", 0.0))
                right = max(scores.get("eyes_r", 0.0), scores.get("lid_r", 0.0))
                eyes = min(left, right)
                # Likewise the mouth reads if either its line or the lit chin under it does.
                mouth_r = max(scores.get("mouth", 0.0), scores.get("chin", 0.0))
                flag = ""
                for label, r in (("eye", eyes), ("mouth", mouth_r)):
                    if r < MIN_RATIO:
                        flag = "  <-- under %.1f (%s)" % (MIN_RATIO, label)
                        failures.append((build, age, tone_name, label, r))
                worst = min(worst, eyes, mouth_r)
                print(
                    "%-11s %-8s %-8s %5.2f  %5.2f%s"
                    % (build, age, tone_name, eyes, mouth_r, flag)
                )
    outline_hex = rgb_to_hex(people.OUTLINE)
    print("\noutline %s   worst %.2f   gate %.1f" % (outline_hex, worst, MIN_RATIO))
    if failures:
        print("\n%d feature(s) below the gate:" % len(failures))
        for f in failures:
            print("   %s %s %s %s %.2f" % f)
        return 1
    print("every face feature reads on every skin tone")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
