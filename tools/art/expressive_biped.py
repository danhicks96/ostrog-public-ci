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


def draw_fitted_hair(pen, direction, style, ink, colours):
    shadow, mid, light, glint = colours
    front = direction == 'S'
    back = direction == 'N'
    # Draw the tied length behind the crown. The face layer is below the hair,
    # so the lock on the far side belongs outside the visible cheek.
    if style == 'ponytail':
        x = 13 if back else 1
        pen.polygon([(x+5,12),(x+11,15),(x+9,22),(x+10,28),(x+6,36),
                     (x+2,40),(x+3,32),(x,33),(x+3,24),(x+1,18)], fill=mid, outline=ink)
        pen.line([(x+5,18),(x+7,22),(x+6,29),(x+4,34)], fill=light, width=2)
    elif style == 'bun':
        x, y = (10, 0) if front else ((10, 4) if back else (0, 5))
        pen.ellipse((x,y,x+14,y+13),fill=mid,outline=ink,width=2)
        pen.arc((x+2,y+2,x+11,y+10),115,325,fill=light,width=2)
    elif style == 'braids':
        for x in ([1,26] if front or back else [2]):
            for y in (18,24,30):
                pen.ellipse((x,y,x+7,y+8),fill=mid,outline=ink)
                pen.line([(x+2,y+1),(x+4,y+3),(x+2,y+6)],fill=light)
            pen.polygon([(x+2,37),(x+5,36),(x+6,41),(x+1,40)],fill=shadow,outline=ink)
    if style == 'balding':
        # The open crown is skin. A low horseshoe frames the back of the head.
        if back:
            pen.polygon([(2,10),(6,12),(8,18),(15,21),(23,20),(28,15),(31,9),
                         (33,17),(30,25),(24,29),(13,29),(6,24)],fill=mid,outline=ink)
            pen.arc((5,5,30,25),24,147,fill=light,width=2)
        else:
            for x in ([2,27] if front else [3]):
                pen.polygon([(x,12),(x+3,10),(x+5,14),(x+4,21),(x+2,23),(x,18)],fill=mid,outline=ink)
                pen.line([(x+2,13),(x+3,17)],fill=light)
            pen.arc((10,3,24,13),197,293,fill=shadow)
        return
    if style == 'crest':
        if front or back:
            outline=[(12,16),(11,8),(14,9),(13,3),(16,5),(18,0),(19,3),(22,1),
                     (21,5),(24,4),(22,13),(19,18 if front else 29),(16,20 if front else 30)]
        else:
            outline=[(2,20),(3,11),(6,13),(5,6),(9,8),(9,2),(13,5),(16,0),
                     (19,4),(24,1),(23,5),(29,3),(27,8),(31,7),(27,11),(17,10),(9,14)]
        pen.polygon(outline,fill=mid,outline=ink)
        pen.line([(16,13),(17,8),(19,4)] if front or back else [(6,13),(13,8),(21,6)],fill=light,width=2)
        return
    crown_y = 4 if style == 'bun' and front else 1
    crown=[(1,17),(2,10),(5,6),(11,crown_y+1),(18,crown_y),(26,crown_y+2),(31,7),(33,13)]
    if back:
        edge=[(32,21),(28,26),(21,29),(13,29),(6,25),(3,22)]
    elif front:
        edge=[(32,18),(29,16),(27,11),(22,9),(16,7),(11,10),(6,12),(4,18)]
    else:
        edge=[(29,12),(24,9),(18,10),(14,15),(11,16),(9,23),(5,25),(2,20)]
    if style == 'fringe' and not back:
        edge=([(32,18),(28,14),(24,14),(22,11),(19,14),(13,14),(14,11),
               (8,14),(5,13),(3,20)] if front else
              [(32,14),(29,13),(25,15),(26,12),(20,14),(16,13),(13,18),(9,21),(6,25),(1,20)])
    pen.polygon(crown+edge,fill=mid,outline=ink)
    if style == 'curly':
        curls = [(2,10),(5,5),(11,2),(18,1),(25,3),(29,7)]
        if back:curls += [(1,16),(3,22),(9,25),(16,26),(23,24),(28,19)]
        elif front:curls += [(1,16),(27,13)]
        else:curls += [(1,16),(4,20)]
        for x,y in curls:
            pen.ellipse((x-1,y-1,x+5,y+5),fill=mid,outline=shadow)
            pen.arc((x,y,x+4,y+3),170,300,fill=light)
    else:
        pen.line([(4,12),(8,7),(14,4),(18,crown_y+1)],fill=light,width=2)
        pen.line([(10,6),(14,4)],fill=glint)
        if style == 'crop':
            for x,y in [(9,9),(15,7),(21,6),(24,9)]:pen.line((x,y,x+2,y-1),fill=shadow)
        elif style == 'fringe':
            pen.line([(24,5),(20,8),(18,12)],fill=light)
            pen.line([(17,5),(14,8),(10,11)],fill=shadow)
        else:
            pen.arc((5,3,29,21),195,286,fill=shadow)
            if back:pen.line([(8,17),(15,20),(24,19)],fill=light)
    if style == 'bun' and back:
        pen.ellipse((10,11,25,25),fill=mid,outline=ink,width=2)
        pen.arc((13,14,23,22),90,333,fill=light,width=2)
    elif style == 'ponytail' and back:
        pen.polygon([(12,14),(17,12),(22,16),(22,23),(20,28),(21,33),
                     (18,39),(13,42),(14,36),(11,37),(13,29),(12,22)],fill=mid,outline=ink)
        pen.line([(15,17),(18,19),(18,25),(16,29),(17,34),(15,37)],fill=light,width=2)
        pen.line([(13,17),(18,16),(21,18)],fill=shadow,width=2)


def expressive_mark(face, direction, pattern, mid, shadow, high):
    """Colour the face's plain skin. Brows, eyes, mouth, shading and outline remain intact."""
    layer = Image.new('RGBA', face.size)
    for y in range(3, 29):
        for x in range(2, 32):
            if face.getpixel((x, y)) != (*mid, 255):
                continue
            if pattern == 'cap' and y <= 8:
                colour = high if y == 8 else shadow
            elif pattern == 'mask' and direction != 'N' and 8 <= y <= 16:
                colour = high if y == 8 else shadow
            elif pattern == 'spots' and y % 5 in (0, 1) and (x + 3 * (y // 5)) % 9 in (0, 1):
                colour = high
            else:
                continue
            layer.putpixel((x, y), (*colour, 255))
    return layer


def make_frame(art, kind, direction, frame, build='average', style='bowl'):
    d = 'E' if direction == 'W' else direction
    pose = (
        art.SLEEP_DRAW[frame]
        if art.asleep(frame)
        else ('idle' if frame == 'lying' else frame)
    )
    s = art.skeleton(build, 'adult', d, pose)
    canvas = art.Canvas()
    if kind not in ('hair', 'marks'):
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
    if kind in ('body', 'marks'):
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
        if kind == 'marks':
            face = expressive_mark(face, d, style, mid, shadow, high)
    elif kind == 'hair':
        if style == 'long':
            draw_long_hair(pen, d, ink, art.SWAP_SOURCES['hair'])
        elif style != 'bowl':
            draw_fitted_hair(pen, d, style, ink, art.SWAP_SOURCES['hair'])
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
    if kind in ('body', 'hair', 'marks'):
        if kind == 'hair' and style == 'bun':
            # Leave space outside the head silhouette for the gathered knot.
            knot = Image.new('RGBA', (46, 52))
            brush = ImageDraw.Draw(knot)
            left = 15 if d in ('S', 'N') else 0
            top = 1 if d in ('S', 'N') else 7
            brush.ellipse((left, top, left+14, top+13), fill=hair_mid, outline=ink, width=2)
            brush.arc((left+2, top+2, left+11, top+9), 160, 316, fill=hair_light, width=2)
            knot.alpha_composite(face, (5, 8))
            face, ox, oy = knot, ox-5, oy-8
        if build == 'slim':
            # Fit the face and hair together to the narrower head on this rig.
            face = face.resize((face.width - 4, face.height), Image.Resampling.NEAREST)
            ox += 2
        image.alpha_composite(face, (ox, oy))
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


def make_sheet(art, kind, build='average', style='bowl'):
    image = Image.new('RGBA', (96 * len(art.FRAMES), 96 * len(art.DIRS)))
    for row, d in enumerate(art.DIRS):
        for column, frame in enumerate(art.FRAMES):
            image.paste(make_frame(art, kind, d, frame, build, style), (column * 96, row * 96))
    return image
