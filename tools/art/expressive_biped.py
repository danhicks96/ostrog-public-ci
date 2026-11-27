"""Two-pixel-per-unit art for layered adult human appearances.

The original 48-unit skeleton drives the head, arms and feet in every pose.
Colours stay in the same skin, hair and cloth swap groups as the other sheets.
"""
from PIL import Image, ImageDraw


def draw_long_hair(pen, direction, ink, colours):
    shadow, mid, light, glint = colours
    # Parted waves descend behind the cheeks and taper over the collar.
    if direction == 'N':
        contour = [(0, 17), (2, 7), (9, 2), (17, 0), (25, 2), (32, 8),
                   (34, 21), (32, 29), (35, 35), (30, 33), (29, 40),
                   (25, 37), (21, 42), (19, 39), (12, 42), (10, 37),
                   (5, 39), (5, 32), (1, 34), (3, 25)]
    elif direction == 'S':
        contour = [(1, 17), (3, 7), (9, 2), (18, 0), (25, 2), (31, 7),
                   (34, 16), (32, 23), (33, 30), (35, 33), (30, 32),
                   (28, 38), (25, 38), (27, 29), (26, 23), (29, 17),
                   (28, 12), (23, 9), (19, 5), (14, 9), (8, 12), (6, 16),
                   (9, 23), (7, 29), (10, 36), (7, 38), (4, 32),
                   (1, 33), (3, 25)]
    else:
        contour = [(0, 18), (2, 8), (8, 3), (16, 0), (24, 1), (30, 5),
                   (32, 9), (26, 7), (21, 8), (17, 12), (14, 14),
                   (14, 20), (12, 25), (14, 32), (11, 38), (9, 35),
                   (6, 40), (5, 34), (1, 36), (3, 27)]
    pen.polygon(contour, fill=mid, outline=ink)
    # A single crown highlight splits into locks that follow each view's contour.
    pen.line([(3, 15), (5, 9), (10, 4), (17, 2)], fill=light, width=2)
    pen.line([(8, 6), (12, 3), (16, 2)], fill=glint)
    pen.line([(4, 16), (5, 21), (6, 25), (5, 30), (6, 34)], fill=shadow, width=2)
    if direction == 'N':
        pen.line([(18, 3), (20, 9), (18, 17), (14, 24), (15, 31), (13, 38)], fill=light, width=2)
        pen.line([(25, 6), (29, 13), (27, 22), (23, 29), (24, 35)], fill=shadow, width=2)
        pen.line([(8, 10), (9, 17), (7, 25)], fill=light)
    elif direction == 'S':
        pen.line([(21, 4), (26, 7), (30, 12), (31, 17)], fill=light, width=2)
        pen.line([(32, 20), (29, 25), (30, 31), (28, 35)], fill=shadow)
    else:
        pen.line([(21, 4), (15, 8), (11, 15), (12, 21), (9, 27), (10, 33)], fill=light)


def make_frame(art, kind, direction, frame, build='average'):
    d = 'E' if direction == 'W' else direction
    pose = (
        art.SLEEP_DRAW[frame]
        if art.asleep(frame)
        else ('idle' if frame == 'lying' else frame)
    )
    s = art.skeleton(build, 'adult', d, pose)
    canvas = art.Canvas()
    if kind != 'hair':
        art.draw_layer(canvas, s, kind, 'vest' if kind == 'clothing' else None, frame)
    image = Image.fromarray(canvas.render()).resize((96, 96), Image.Resampling.NEAREST)
    x0, y0, x1, y1 = s.head
    # Local coordinates of the 34 x 33 head. Its chin meets the original neck.
    ox = x0 + x1 - 16
    oy = 2 * y1 - 29
    face = Image.new('RGBA', (36, 44))
    pen = ImageDraw.Draw(face)
    ink = art.pal.neutral('shadow')
    shadow, mid, light, high = art.SWAP_SOURCES['skin']
    hair_shadow, hair_mid, hair_light, hair_glint = art.SWAP_SOURCES['hair']
    if kind == 'body':
        # Remove the old head, including its outline. The arms are retained below it.
        ImageDraw.Draw(image).rectangle(
            (2 * x0 - 3, 2 * y0 - 2, 2 * x1 + 5, 2 * y1 + 2), fill=(0, 0, 0, 0)
        )
        if d == 'E':
            pen.polygon(
                [
                    (4, 7),
                    (10, 2),
                    (20, 1),
                    (27, 5),
                    (29, 12),
                    (29, 16),
                    (33, 19),
                    (32, 21),
                    (29, 22),
                    (28, 26),
                    (23, 30),
                    (16, 30),
                    (9, 26),
                    (5, 20),
                ],
                fill=shadow,
                outline=ink,
            )
            pen.polygon(
                [
                    (7, 9),
                    (13, 3),
                    (21, 3),
                    (26, 7),
                    (27, 15),
                    (27, 18),
                    (31, 20),
                    (28, 21),
                    (26, 25),
                    (22, 28),
                    (16, 28),
                    (10, 24),
                ],
                fill=mid,
            )
            pen.ellipse((9, 14, 15, 22), fill=light)
            pen.line([(10, 18), (12, 16), (13, 18), (11, 20)], fill=shadow)
            pen.line([(22, 24), (25, 24), (27, 23)], fill=shadow)
            eyes = [(22, 13)]
        else:
            pen.polygon(
                [
                    (2, 10),
                    (5, 5),
                    (11, 2),
                    (22, 2),
                    (29, 6),
                    (32, 12),
                    (31, 20),
                    (27, 26),
                    (21, 30),
                    (14, 31),
                    (8, 28),
                    (4, 23),
                    (2, 19),
                ],
                fill=shadow,
                outline=ink,
            )
            pen.polygon(
                [
                    (4, 11),
                    (8, 6),
                    (15, 3),
                    (24, 5),
                    (28, 8),
                    (30, 14),
                    (28, 21),
                    (25, 25),
                    (19, 29),
                    (14, 29),
                    (8, 25),
                    (5, 20),
                ],
                fill=mid,
            )
            if d == 'S':
                pen.ellipse((5, 16, 11, 22), fill=light)
                pen.ellipse((24, 17, 28, 21), fill=light)
                pen.line([(16, 17), (15, 20), (17, 21), (19, 20)], fill=light)
                pen.point((17, 21), fill=shadow)
                pen.line([(13, 24), (16, 25), (20, 24)], fill=shadow)
                pen.line([(15, 27), (18, 27)], fill=light)
            eyes = [(7, 13), (21, 13)] if d == 'S' else []
        if build == 'broad' and d == 'S':
            # A wider jaw and straight brows retain the broader rig's silhouette.
            pen.polygon([(3, 20), (8, 24), (13, 27), (21, 27), (28, 23),
                         (31, 19), (30, 25), (23, 30), (14, 31), (6, 27)],
                        fill=shadow)
            pen.line([(9, 25), (14, 28), (20, 28), (26, 25)], fill=mid, width=2)
            pen.line([(7, 10), (12, 10)], fill=shadow, width=2)
            pen.line([(21, 10), (26, 9)], fill=shadow, width=2)
        for ex, ey in eyes:
            pen.line(
                [(ex - 1, ey - 2), (ex + 1, ey - 4), (ex + 4, ey - 3)], fill=shadow
            )
            if art.asleep(frame) or frame == 'lying':
                pen.line([(ex, ey + 1), (ex + 2, ey + 2), (ex + 4, ey + 1)], fill=ink)
            else:
                pen.ellipse((ex, ey, ex + 5, ey + 5), fill=art.pal.neutral('light'))
                iris = ex + (2 if frame == 'glance' or d == 'E' else 1)
                pen.ellipse((iris, ey, iris + 3, ey + 4), fill=ink)
                pen.point((iris + 1, ey + 1), fill=art.pal.neutral('white'))
                pen.line([(ex, ey + 1), (ex + 2, ey - 1), (ex + 5, ey)], fill=ink)
        image.alpha_composite(face, (ox, oy))
    elif kind == 'hair':
        if build == 'broad':
            draw_long_hair(pen, d, ink, art.SWAP_SOURCES['hair'])
        elif d == 'N':
            pen.polygon(
                [
                    (1, 11),
                    (3, 5),
                    (9, 1),
                    (16, 0),
                    (23, 1),
                    (29, 5),
                    (33, 11),
                    (33, 19),
                    (30, 26),
                    (25, 30),
                    (21, 29),
                    (18, 32),
                    (13, 30),
                    (9, 31),
                    (7, 28),
                    (4, 27),
                    (2, 22),
                ],
                fill=hair_shadow,
                outline=ink,
            )
            pen.ellipse((4, 2, 30, 25), fill=hair_mid)
            pen.arc((7, 2, 24, 28), 158, 260, fill=hair_light, width=2)
            pen.arc((8, 0, 29, 26), 282, 355, fill=hair_light)
            pen.arc((6, 2, 20, 29), 110, 189, fill=hair_shadow)
        else:
            edge = (
                [
                    (32, 18),
                    (29, 18),
                    (28, 12),
                    (23, 10),
                    (19, 11),
                    (22, 6),
                    (16, 9),
                    (10, 12),
                    (12, 8),
                    (7, 12),
                    (5, 20),
                    (3, 23),
                    (1, 18),
                ]
                if d == 'S'
                else [
                    (29, 11),
                    (25, 9),
                    (20, 12),
                    (22, 8),
                    (16, 12),
                    (14, 16),
                    (10, 16),
                    (9, 22),
                    (7, 26),
                    (4, 24),
                    (3, 26),
                    (1, 20),
                ]
            )
            pen.polygon(
                [(1, 12), (3, 6), (7, 3), (13, 0), (21, 0), (27, 3), (31, 7)] + edge,
                fill=hair_mid,
                outline=ink,
            )
            pen.polygon(
                [(2, 13), (4, 8), (10, 4), (15, 3), (11, 6), (6, 10), (4, 16)],
                fill=hair_light,
            )
            pen.line([(8, 6), (12, 3), (16, 2)], fill=hair_glint)
            pen.line([(18, 2), (23, 3), (26, 5)], fill=hair_light)
            pen.line([(20, 5), (16, 6), (11, 10)], fill=hair_shadow)
        image.alpha_composite(face, (ox, oy))
    elif kind == 'clothing':
        # Stitched lapels and belt follow the torso while sleeves follow the original hands.
        tx, ty, rx, by = [2 * v for v in s.torso]
        cloth_dark, cloth_mid, cloth_light, _ = art.SWAP_SOURCES['cloth']
        trim_dark, trim_mid, trim_light, _ = art.SWAP_SOURCES['trim']
        pen = ImageDraw.Draw(image)
        if d == 'S':
            pen.polygon(
                [
                    (tx + 3, ty + 1),
                    (tx + 8, ty + 1),
                    (tx + 13, ty + 10),
                    (tx + 9, ty + 8),
                ],
                fill=cloth_light,
            )
            pen.line(
                [(rx - 4, ty + 2), (tx + 13, ty + 10), (tx + 13, by - 2)],
                fill=cloth_dark,
            )
            for y in range(ty + 12, by - 2, 4):
                pen.point((tx + 15, y), fill=trim_light)
        pen.rectangle((tx + 2, by - 4, rx - 2, by - 2), fill=trim_dark)
        pen.line((tx + 3, by - 4, rx - 3, by - 4), fill=trim_mid)
        if d != 'N':
            pen.rectangle((rx - 9, by - 5, rx - 6, by - 2), outline=trim_light)
    shifts = art.lean_shifts(s)
    if shifts:
        moved = Image.new('RGBA', image.size)
        for row in range(96):
            moved.paste(
                image.crop((0, row, 96, row + 1)), (2 * shifts.get(row // 2, 0), row)
            )
        image = moved
    if art.asleep(frame) or frame == 'lying':
        image = image.transpose(Image.Transpose.ROTATE_90)
    if direction == 'W':
        image = image.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
    return image


def make_sheet(art, kind, build='average'):
    image = Image.new('RGBA', (96 * len(art.FRAMES), 96 * len(art.DIRS)))
    for row, d in enumerate(art.DIRS):
        for column, frame in enumerate(art.FRAMES):
            image.paste(make_frame(art, kind, d, frame, build), (column * 96, row * 96))
    return image
