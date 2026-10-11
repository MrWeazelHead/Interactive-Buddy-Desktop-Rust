"""Generate companion-pack art at the exact corresponding base sprite dimensions.
Run from any directory with Pillow installed. No original assets are modified.
"""
from pathlib import Path
from PIL import Image, ImageDraw
import json

ROOT = Path(__file__).resolve().parents[1] / 'rewrite' / 'assets'
manifest = {}
for skin, color in [('robot', (125, 160, 185)), ('cosmic', (96, 62, 170)), ('slime', (95, 205, 100)),
                    ('neon', (75, 215, 175)), ('ice', (135, 215, 250)), ('ember', (235, 100, 40))]:
    for part in ['head', 'body', 'rArm', 'lArm', 'rLeg', 'lLeg']:
        base = Image.open(ROOT / 'skins' / f'default_{part}.png').convert('RGBA')
        pixels = []
        for r, g, b, a in base.getdata():
            light = (r + g + b) / 765
            pixels.append(tuple(int(c * (0.3 + 0.7 * light)) for c in color) + (a,))
        out = Image.new('RGBA', base.size)
        out.putdata(pixels)
        d = ImageDraw.Draw(out)
        box = base.getchannel('A').getbbox()
        if box:
            x0, y0, x1, y1 = box
            cx, cy = (x0 + x1) // 2, (y0 + y1) // 2
            if skin in ('neon', 'ice', 'ember'):
                accent = {'neon': '#ff65d5', 'ice': '#e8fbff', 'ember': '#ffd16b'}[skin]
                if part == 'head':
                    for x in [cx-5, cx+5]:
                        d.ellipse((x-2, cy-4, x+2, cy), fill='#263748')
                    d.arc((cx-5, cy, cx+5, cy+6), 0, 180, fill=accent, width=2)
                if skin == 'neon':
                    d.line((x0+4, cy+5, cx, cy-5, x1-4, cy+5), fill=accent, width=2)
                elif skin == 'ice':
                    d.line((cx-5, cy+8, cx+3, cy+2, cx-2, cy-4), fill=accent, width=2)
                    d.line((cx+3, cy+2, cx+7, cy+5), fill=accent)
                else:
                    d.polygon([(cx-6,cy+9),(cx-3,cy+2),(cx,cy+5),(cx+3,cy-3),(cx+6,cy+9)], fill=accent)
                out.putalpha(base.getchannel('A'))
            elif part == 'head':
                if skin == 'robot':
                    d.rectangle((cx-8, cy-4, cx+8, cy+2), fill=(24, 39, 54, 255))
                    for x in [cx-5, cx+5]:
                        d.rectangle((x-1, cy-2, x+1, cy), fill=(100, 240, 255, 255))
                elif skin == 'slime':
                    for x in [cx-5, cx+5]:
                        d.ellipse((x-2, cy-4, x+2, cy+1), fill=(20, 50, 25, 255))
                    d.arc((cx-5, cy, cx+5, cy+6), 0, 180, fill=(20, 50, 25, 255))
                else:
                    d.ellipse((cx-7, cy-4, cx+7, cy+4), fill=(18, 15, 50, 255), outline=(150, 215, 255, 255))
                    d.line((cx-3, cy-2, cx+3, cy-2), fill=(220, 245, 255, 255))
            elif part == 'body':
                if skin == 'robot':
                    d.rectangle((cx-7, cy-6, cx+7, cy+7), fill=(35, 50, 65, 255), outline=(190, 210, 220, 255))
                    d.rectangle((cx-4, cy-3, cx+4, cy), fill=(100, 240, 255, 255))
                elif skin == 'cosmic':
                    for dx, dy in [(-8,-5), (6,4), (0,-9), (-3,9)]:
                        d.line((cx+dx-2, cy+dy, cx+dx+2, cy+dy), fill=(240, 225, 130, 255))
                        d.line((cx+dx, cy+dy-2, cx+dx, cy+dy+2), fill=(240, 225, 130, 255))
                else:
                    d.ellipse((cx-6, cy-9, cx-2, cy-3), fill=(195, 250, 160, 220))
        path = ROOT / 'skins' / f'{skin}_{part}.png'
        out.save(path)
        manifest[path.relative_to(ROOT).as_posix()] = {'size': out.size, 'base': f'skins/default_{part}.png'}

for name, base_name in [('beachball', 'bowlball'), ('anvil', 'radio'), ('bubblewand', 'stunGun'), ('repulsor', 'vortex'), ('confetti', 'pistol')]:
    size = Image.open(ROOT / 'tools' / f'{base_name}.png').size
    im = Image.new('RGBA', size)
    d = ImageDraw.Draw(im)
    w, h = size
    if name == 'beachball':
        for i, col in enumerate(['#ef6262', '#f8d364', '#6acced', '#faf8ed']):
            d.pieslice((1,1,w-2,h-2), i*90, (i+1)*90, fill=col)
        d.ellipse((1,1,w-2,h-2), outline='#263748')
        d.ellipse((w//2-2,h//2-2,w//2+2,h//2+2), fill='white')
    elif name == 'anvil':
        d.polygon([(1,5),(w-2,5),(w-6,11),(15,11),(15,17),(w-4,20),(3,20),(8,17),(8,11),(4,10)], fill='#77879a', outline='#263748')
        d.line((3,6,w-4,6), fill='#c1d1dc')
    elif name == 'bubblewand':
        d.ellipse((2,1,14,h-2), outline='#91e7fa', width=2)
        d.rectangle((14,h//2-1,w-2,h//2+2), fill='#e799d7')
    elif name == 'repulsor':
        for inset in [2,7,12]:
            d.ellipse((inset,inset,w-1-inset,h-1-inset), outline='#ed9b65', width=2)
        d.line((w//2,4,w//2,h-5), fill='#ffebbc', width=2)
    else:
        d.polygon([(2,7),(25,7),(33,4),(33,13),(24,13),(15,15),(12,21),(7,21),(9,14),(2,14)], fill='#ac6dd4', outline='#332640')
        for x, col in [(5,'#ffcf66'),(12,'#ef759e'),(19,'#79dcce')]:
            d.rectangle((x,8,x+3,12), fill=col)
    path = ROOT / 'tools' / f'{name}.png'
    im.save(path)
    assert im.size == Image.open(ROOT / 'tools' / f'{base_name}.png').size
    manifest[path.relative_to(ROOT).as_posix()] = {'size': size, 'base': f'tools/{base_name}.png'}
# Preserve extracted originals; custom gun and fireball keys need renderer wiring.
for name, base_name in [
    ('tomato', 'baseball'), ('basketball', 'baseball'), ('snowball', 'baseball'),
    ('waterballoon', 'baseball'), ('marble', 'rubberball'),
    ('cherrybomb', 'grenade'), ('bomb', 'grenade'), ('dynamite', 'grenade'),
    ('firecracker', 'grenade'), ('fireball_custom', 'fireball'),
    ('machinegun_custom', 'machineGun'), ('shotgun_custom', 'shotgun'),
    ('duck', 'radio'), ('icewand', 'stunGun'), ('sparkler', 'stunGun'),
]:
    size = Image.open(ROOT / 'tools' / f'{base_name}.png').size
    # Draw on a shared normalized canvas, then antialias to native dimensions.
    im = Image.new('RGBA', (100, 100))
    d = ImageDraw.Draw(im)
    ink = '#263748'
    if name in ('tomato', 'basketball', 'snowball', 'marble', 'cherrybomb', 'bomb'):
        color = {'tomato': '#ef5546', 'basketball': '#ed963c', 'snowball': '#e8f5ff',
                 'marble': '#68d5cb', 'cherrybomb': '#ce354b', 'bomb': '#434956'}[name]
        box = (8, 22, 92, 94) if name in ('cherrybomb', 'bomb') else (6, 8, 94, 94)
        d.ellipse(box, fill=color, outline=ink, width=5)
        d.arc((17, 18, 78, 81), 190, 265, fill='#ffffff', width=5)
        if name == 'tomato':
            d.polygon([(50,8),(57,22),(76,17),(63,30),(70,39),(50,31),(30,39),(37,25),(26,17),(44,22)], fill='#49834c')
        elif name == 'basketball':
            d.line((50,10,50,92), fill=ink, width=4)
            d.line((8,50,92,50), fill=ink, width=4)
            d.arc((15,-8,80,109), 270, 90, fill=ink, width=4)
        elif name == 'snowball':
            for x, y in [(28,60),(65,75),(78,42)]:
                d.ellipse((x,y,x+5,y+5), fill='#9ac5df')
        elif name == 'marble':
            d.arc((23,17,77,84), 20, 240, fill='#775dc9', width=12)
        elif name in ('cherrybomb', 'bomb'):
            d.line((52,23,58,10,77,10), fill='#c3a572', width=5)
            d.line((73,3,81,16), fill='#ffbb46', width=5)
    elif name == 'waterballoon':
        d.ellipse((14,7,86,84), fill='#64bde9', outline=ink, width=5)
        d.polygon([(43,80),(57,80),(64,94),(36,94)], fill='#64bde9', outline=ink)
        d.arc((24,17,72,72), 190, 260, fill='#d7f5ff', width=6)
    elif name in ('dynamite', 'firecracker'):
        d.line((50,22,54,8,72,6), fill='#c3a572', width=5)
        d.line((67,2,77,12), fill='#ffc954', width=5)
        d.rounded_rectangle((23,22,77,95), radius=7, fill='#dc5949', outline=ink, width=5)
        if name == 'dynamite':
            for x in (38,61):
                d.line((x,26,x,91), fill='#943d3b', width=4)
            d.rectangle((23,48,77,63), fill='#d8c399', outline=ink, width=3)
        else:
            d.rectangle((26,32,74,44), fill='#ffd16b')
            d.rectangle((26,75,74,87), fill='#ffd16b')
    elif name == 'fireball_custom':
        d.polygon([(15,83),(9,57),(24,31),(30,53),(56,4),(61,43),(81,25),(94,65),(83,91),(44,97)], fill='#ee7136', outline=ink, width=4)
        d.polygon([(29,81),(40,57),(45,70),(59,38),(71,78),(59,90)], fill='#ffde72')
    elif name in ('machinegun_custom', 'shotgun_custom'):
        if name == 'machinegun_custom':
            d.polygon([(5,33),(26,33),(31,27),(66,27),(66,35),(95,35),(95,47),(66,47),(62,58),(52,58),(55,87),(40,87),(37,56),(25,56),(13,78),(5,78)], fill='#647b83', outline=ink, width=4)
            d.rectangle((69,28,92,52), outline=ink, width=4)
            d.line((31,35,60,35), fill='#b4ced0', width=4)
        else:
            d.polygon([(5,40),(35,40),(39,29),(95,29),(95,43),(65,43),(65,57),(35,57),(14,84),(5,80)], fill='#a7754f', outline=ink, width=4)
            d.rectangle((39,29,95,40), fill='#8b9aa4', outline=ink, width=3)
            d.line((46,45,46,54,58,54,58,45), fill=ink, width=4)
    elif name == 'duck':
        d.ellipse((8,42,85,91), fill='#f6d75e', outline=ink, width=4)
        d.ellipse((50,10,86,52), fill='#ffe477', outline=ink, width=4)
        d.polygon([(82,28),(98,34),(82,41)], fill='#ed963c', outline=ink)
        d.ellipse((72,23,78,29), fill=ink)
        d.arc((22,46,68,79), 0, 170, fill='#d89c36', width=4)
    else:
        d.line((10,85,68,35), fill='#779aa6' if name == 'icewand' else '#9faaa9', width=7)
        if name == 'icewand':
            d.polygon([(68,5),(87,27),(77,58),(56,42),(49,20)], fill='#9ee4ff', outline=ink, width=4)
            d.line((68,8,66,34,77,54), fill='#edfcff', width=4)
        else:
            for end in [(70,4),(94,16),(96,47),(72,62),(47,42),(45,9)]:
                d.line((71,31,*end), fill='#ffca59', width=4)
            d.ellipse((65,25,77,37), fill='#fff1c2')
    im = im.resize(size, Image.Resampling.LANCZOS)
    assert im.size == size and im.getchannel('A').getbbox(), name
    path = ROOT / 'tools' / f'{name}.png'
    im.save(path)
    manifest[path.relative_to(ROOT).as_posix()] = {'size': size, 'base': f'tools/{base_name}.png'}

for name, base_name in [('foodbowl', 'radio'), ('waterbowl', 'radio'), ('toybasket', 'radio'), ('petbed', 'radio'), ('featherwand', 'stunGun'), ('petbrush', 'stunGun')]:
    size = Image.open(ROOT / 'tools' / f'{base_name}.png').size
    im = Image.new('RGBA', (100, 100))
    d = ImageDraw.Draw(im)
    if name in ('foodbowl', 'waterbowl'):
        d.polygon([(8,42),(92,42),(80,86),(20,86)], fill='#ce8a6a' if name == 'foodbowl' else '#78a9db', outline='#263748', width=4)
        d.ellipse((8,32,92,55), fill='#ad7642' if name == 'foodbowl' else '#68d5ed', outline='#263748', width=4)
        if name == 'foodbowl':
            for x,y in [(24,38),(41,33),(56,41),(71,36),(41,44)]:
                d.ellipse((x,y,x+10,y+8), fill='#e5b363', outline='#6b4936')
        else:
            d.arc((22,35,76,48), 180, 330, fill='#ecfbff', width=3)
    elif name == 'toybasket':
        d.rounded_rectangle((12,45,90,90), radius=7, fill='#b88052', outline='#263748', width=4)
        d.ellipse((20,15,53,55), fill='#ee789e', outline='#263748', width=3)
        d.ellipse((54,26,88,59), fill='#78d5c6', outline='#263748', width=3)
        for x in (28,48,68):
            d.line((x,56,x,86), fill='#e4b67e', width=4)
    elif name == 'petbed':
        d.ellipse((4,24,96,92), fill='#a583cb', outline='#263748', width=4)
        d.ellipse((18,36,82,78), fill='#e4ccf4', outline='#70548d', width=3)
        d.ellipse((29,46,43,57), fill='#a583cb')
        d.ellipse((53,46,67,57), fill='#a583cb')
        d.ellipse((39,58,57,71), fill='#a583cb')
    elif name == 'featherwand':
        d.line((5,79,73,30), fill='#bd9470', width=7)
        d.ellipse((57,7,97,47), fill='#f0a7d8', outline='#754567', width=3)
        d.line((62,39,91,15), fill='#fff0f8', width=3)
    else:
        d.line((10,75,57,41), fill='#ac8565', width=12)
        d.rounded_rectangle((48,16,93,49), radius=8, fill='#88bd98', outline='#263748', width=4)
        for x in (55,66,77,88):
            d.line((x,19,x,39), fill='#f0efcf', width=3)
    im = im.resize(size, Image.Resampling.LANCZOS)
    path = ROOT / 'tools' / f'{name}.png'
    im.save(path)
    manifest[path.relative_to(ROOT).as_posix()] = {'size': size, 'base': f'tools/{base_name}.png'}

for name, base_name in [('petcage','radio'),('tractorbeam','pistol'),('launchpad','radio'),('portalgun','pistol'),('grapple','pistol'),('stasis','stunGun'),('bazooka_launcher','pistol'),('bazooka_rocket','missile'),('littertray','radio'),('poopscoop','stunGun'),('petsoap','radio'),('petmedicine','radio'),('poop','baseball')]:
    size = Image.open(ROOT / 'tools' / f'{base_name}.png').size
    im = Image.new('RGBA', (100, 100)); d = ImageDraw.Draw(im)
    if name in ('portalgun','grapple','bazooka_launcher'):
        d.rounded_rectangle((7,28,91,59), radius=9, fill={'portalgun':'#d9e3ed','grapple':'#74828a','bazooka_launcher':'#758253'}[name], outline='#263748', width=4)
        d.polygon([(24,57),(45,57),(38,91),(21,88)], fill='#41484f', outline='#263748')
        if name=='portalgun':
            d.ellipse((50,33,80,55),fill='#58d4ef',outline='#eeb463',width=4)
            d.line((85,20,91,28,91,59,85,68),fill='#c8d5e3',width=4)
        elif name=='grapple':
            d.arc((75,13,97,73),-90,90,fill='#d9e6ed',width=6)
            d.ellipse((13,34,37,53),fill='#d7a65b',outline='#263748',width=3)
        else:
            d.rectangle((82,20,97,67),fill='#3c4c35',outline='#263748',width=3)
            d.rectangle((41,17,52,27),fill='#c9c48e')
    elif name=='petcage':
        d.rounded_rectangle((6,8,94,91),radius=6,outline='#6ea3af',width=6)
        for x in (21,36,51,66,81):d.line((x,12,x,87),fill='#b7dbe0',width=4)
        d.rectangle((8,78,92,88),fill='#44777e')
        d.rectangle((38,43,65,56),fill='#d9ad55',outline='#674d32',width=3)
    elif name=='tractorbeam':
        d.rounded_rectangle((10,35,84,63),radius=8,fill='#427c78',outline='#263748',width=4)
        d.polygon([(25,60),(44,60),(36,88),(21,85)],fill='#344f54')
        for x in (58,72,86):d.ellipse((x,22,x+10,74),fill='#7cf3c9',outline='#286b69',width=3)
        d.ellipse((15,43,28,55),fill='#c0fff0')
    elif name=='launchpad':
        d.polygon([(10,50),(83,50),(95,69),(5,69)],fill='#9ceab4',outline='#31555d',width=4)
        d.rectangle((10,69,90,86),fill='#3c7d79',outline='#263748',width=4)
        d.polygon([(38,46),(38,30),(29,30),(50,10),(71,30),(62,30),(62,46)],fill='#d8ff82',outline='#52754c',width=3)
    elif name=='bazooka_rocket':
        d.polygon([(50,3),(67,26),(66,78),(79,94),(55,85),(45,85),(21,94),(34,78),(33,26)],fill='#8ca263',outline='#263748',width=4)
        d.polygon([(50,3),(65,27),(35,27)],fill='#e1ad58');d.rectangle((37,40,62,49),fill='#e6d58b')
    elif name=='stasis':
        d.rounded_rectangle((8,36,91,71),radius=8,fill='#7764a5',outline='#263748',width=4)
        d.ellipse((53,16,95,58),fill='#9fe4ed',outline='#ece4ff',width=5)
        d.line((66,28,66,47),fill='#70548d',width=4);d.line((80,28,80,47),fill='#70548d',width=4)
    elif name=='littertray':
        d.polygon([(9,33),(90,33),(80,87),(20,87)],fill='#80aea2',outline='#263748',width=4)
        d.ellipse((8,23,92,48),fill='#d8caa5',outline='#263748',width=4)
        for x,y in [(22,30),(36,35),(56,28),(73,35)]:d.ellipse((x,y,x+4,y+3),fill='#a69674')
    elif name=='poopscoop':
        d.line((5,87,59,38),fill='#7bacbf',width=12)
        d.polygon([(45,21),(84,7),(96,33),(67,56)],fill='#a4d2df',outline='#263748',width=4)
        for x in (61,72,83):d.line((x,20,x+5,36),fill='#40566b',width=3)
    elif name=='petsoap':
        d.rounded_rectangle((12,32,88,80),radius=16,fill='#f4bfd6',outline='#8c5d78',width=4)
        d.ellipse((34,44,64,67),fill='#fae4ed');d.ellipse((65,8,83,26),fill='#b6ebf3',outline='#6ea9bc',width=2)
    elif name=='petmedicine':
        d.rounded_rectangle((23,26,77,91),radius=8,fill='#eff0d6',outline='#263748',width=4)
        d.rectangle((28,8,72,27),fill='#728eaa',outline='#263748',width=3)
        d.rectangle((44,41,57,78),fill='#d95c6b');d.rectangle((32,53,69,66),fill='#d95c6b')
    else:
        for box in [(9,53,91,89),(21,33,79,69),(34,12,64,47)]:d.ellipse(box,fill='#936640',outline='#523c2d',width=3)
        d.ellipse((37,37,45,46),fill='#28262a');d.ellipse((57,37,65,46),fill='#28262a')
    path = ROOT / 'tools' / f'{name}.png'; im.resize(size,Image.Resampling.LANCZOS).save(path)
    manifest[path.relative_to(ROOT).as_posix()] = {'size':size,'base':f'tools/{base_name}.png'}

(ROOT / 'content_manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(f'Generated {len(manifest)} assets; exact base dimensions preserved.')
