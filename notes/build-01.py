# -*- coding: utf-8 -*-
from reportlab.lib.pagesizes import A4
from reportlab.lib.units import mm
from reportlab.lib import colors
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.enums import TA_LEFT
from reportlab.platypus import (BaseDocTemplate, PageTemplate, Frame, Paragraph, KeepTogether,
                                Spacer, Table, TableStyle)

OUT = r"E:/Personal Project/BlockChain Network/notes/01-mining-and-the-block-header.pdf"

INK    = colors.HexColor("#1a1a1a")
MUTED  = colors.HexColor("#6b6b6b")
RULE   = colors.HexColor("#d8d4cc")
ACCENT = colors.HexColor("#b8621b")
BAND   = colors.HexColor("#f4f1ea")
CODEBG = colors.HexColor("#f7f5f0")

ss = getSampleStyleSheet()


def S(name, **kw):
    base = kw.pop("parent", ss["Normal"])
    return ParagraphStyle(name, parent=base, **kw)


TitleS = S("TitleS", fontName="Helvetica-Bold", fontSize=23, leading=27,
           textColor=INK, spaceAfter=2)
SubTitleS = S("SubTitleS", fontName="Helvetica", fontSize=10.5, leading=14,
              textColor=MUTED, spaceAfter=0)
H1 = S("H1", fontName="Helvetica-Bold", fontSize=13.5, leading=17, textColor=INK,
       spaceBefore=17, spaceAfter=6)
H2 = S("H2", fontName="Helvetica-Bold", fontSize=10.8, leading=14, textColor=ACCENT,
       spaceBefore=11, spaceAfter=4)
Body = S("Body", fontName="Helvetica", fontSize=9.7, leading=14.2, textColor=INK,
         spaceAfter=6, alignment=TA_LEFT)
Small = S("Small", fontName="Helvetica", fontSize=8.6, leading=12, textColor=MUTED,
          spaceAfter=4)
Code = S("Code", fontName="Courier", fontSize=8.6, leading=12.4, textColor=INK,
         spaceAfter=0, spaceBefore=0)
CellB = S("CellB", fontName="Helvetica", fontSize=8.5, leading=11.6, textColor=INK)
CellH = S("CellH", fontName="Helvetica-Bold", fontSize=8.5, leading=11.6, textColor=INK)
Note = S("Note", fontName="Helvetica", fontSize=9.2, leading=13.4, textColor=INK,
         spaceAfter=0)


def rule(space_before=3, space_after=9):
    t = Table([[""]], colWidths=[165 * mm], rowHeights=[0.5])
    t.setStyle(TableStyle([("LINEBELOW", (0, 0), (-1, -1), 0.6, RULE),
                           ("TOPPADDING", (0, 0), (-1, -1), 0),
                           ("BOTTOMPADDING", (0, 0), (-1, -1), 0)]))
    return [Spacer(1, space_before), t, Spacer(1, space_after)]


def codeblock(lines):
    rows = [[Paragraph(l.replace(" ", "&nbsp;") or "&nbsp;", Code)] for l in lines]
    t = Table(rows, colWidths=[165 * mm])
    t.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), CODEBG),
        ("LEFTPADDING", (0, 0), (-1, -1), 9),
        ("RIGHTPADDING", (0, 0), (-1, -1), 9),
        ("TOPPADDING", (0, 0), (-1, -1), 1.5),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 1.5),
        ("LINEBEFORE", (0, 0), (0, -1), 2, ACCENT),
    ]))
    return [Spacer(1, 4), t, Spacer(1, 9)]


def callout(title, text):
    inner = [Paragraph("<b>%s</b>" % title, Note), Spacer(1, 3), Paragraph(text, Note)]
    t = Table([[inner]], colWidths=[165 * mm])
    t.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), BAND),
        ("LEFTPADDING", (0, 0), (-1, -1), 11),
        ("RIGHTPADDING", (0, 0), (-1, -1), 11),
        ("TOPPADDING", (0, 0), (-1, -1), 9),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 9),
        ("LINEBEFORE", (0, 0), (0, -1), 2.5, ACCENT),
    ]))
    return [Spacer(1, 5), t, Spacer(1, 10)]


def datatable(header, rows, widths, aligns=None):
    data = [[Paragraph(h, CellH) for h in header]]
    for r in rows:
        data.append([c if isinstance(c, Paragraph) else Paragraph(str(c), CellB) for c in r])
    t = Table(data, colWidths=widths, repeatRows=1)
    st = [
        ("BACKGROUND", (0, 0), (-1, 0), BAND),
        ("LINEBELOW", (0, 0), (-1, 0), 0.8, INK),
        ("LINEBELOW", (0, 1), (-1, -2), 0.35, RULE),
        ("VALIGN", (0, 0), (-1, -1), "TOP"),
        ("LEFTPADDING", (0, 0), (-1, -1), 6),
        ("RIGHTPADDING", (0, 0), (-1, -1), 6),
        ("TOPPADDING", (0, 0), (-1, -1), 5),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 5),
    ]
    if aligns:
        for col, a in aligns.items():
            st.append(("ALIGN", (col, 0), (col, -1), a))
    t.setStyle(TableStyle(st))
    return t


story = []
A = story.append
E = story.extend

A(Paragraph("Mining and the Block Header", TitleS))
A(Paragraph("Rust Miner project &nbsp;&middot;&nbsp; Notes 01 &nbsp;&middot;&nbsp; 6 September 2026",
            SubTitleS))
E(rule(6, 12))

A(Paragraph(
    "These notes cover what mining actually computes, and the exact structure of the 80 bytes "
    "that get hashed. This is the reference for Stage 2 of the roadmap, where the toy "
    "string-plus-counter is replaced by a real Bitcoin block header.", Body))

# 1
A(Paragraph("1 &nbsp; What mining actually is", H1))
A(Paragraph(
    "Mining is guessing. A miner assembles a block, hashes its 80-byte header, and checks "
    "whether the resulting number is below a threshold called the <b>target</b>. It almost "
    "never is, so the miner changes one field and hashes again. Billions of times per second, "
    "by every miner on the network at once.", Body))
E(codeblock([
    "loop {",
    "    header.nonce += 1;",
    "    let h = sha256(sha256(header));",
    "    if h < target {",
    "        broadcast(block);   // you won",
    "        break;",
    "    }",
    "}",
]))
A(Paragraph(
    "That is the whole algorithm. Increment, hash, compare. Nothing is being solved and there "
    "is no partial progress: a miner that has guessed a trillion times is no closer than one "
    "that just started. Every guess is independent.", Body))

E(callout("Why this is worth doing at all",
          "Hashing is hard to do and trivial to check. Producing a hash below the target costs "
          "real electricity; verifying someone else&rsquo;s claim costs one hash. That asymmetry "
          "is what converts electricity into un-forgeable votes, which is what stops an attacker "
          "from rewriting history. The rare hash is a receipt for energy burned."))

# 2
A(Paragraph("2 &nbsp; The 80-byte header", H1))
A(Paragraph(
    "Six fields, always in this order, always exactly 80 bytes regardless of how many "
    "transactions the block contains.", Body))

fixed = '<font color="#8a3324"><b>No</b></font>'
varies = '<font color="#2c6e49"><b>Yes</b></font>'
partly = '<font color="#6b6b6b">Barely</font>'
indirect = '<font color="#2c6e49"><b>Indirectly</b></font>'

hdr_table = datatable(
    ["#", "Field", "Size", "What it holds", "Miner<br/>varies?"],
    [
        ["1", Paragraph("<font face='Courier'>version</font>", CellB), "4",
         "Protocol version, plus bits used to signal support for soft forks.",
         Paragraph(partly, CellB)],
        ["2", Paragraph("<font face='Courier'>prev_block_hash</font>", CellB), "32",
         "Hash of the previous block&rsquo;s header. This single field is the entire "
         "&ldquo;chain&rdquo; in blockchain.", Paragraph(fixed, CellB)],
        ["3", Paragraph("<font face='Courier'>merkle_root</font>", CellB), "32",
         "One hash summarising every transaction in the block.", Paragraph(indirect, CellB)],
        ["4", Paragraph("<font face='Courier'>timestamp</font>", CellB), "4",
         "Unix time in seconds, self-reported by the miner.", Paragraph(varies, CellB)],
        ["5", Paragraph("<font face='Courier'>nBits</font>", CellB), "4",
         "The target, in a compressed 4-byte encoding.", Paragraph(fixed, CellB)],
        ["6", Paragraph("<font face='Courier'>nonce</font>", CellB), "4",
         "A meaningless counter that exists only to be changed.", Paragraph(varies, CellB)],
    ],
    widths=[8 * mm, 34 * mm, 14 * mm, 87 * mm, 22 * mm],
    aligns={0: "CENTER", 2: "CENTER", 4: "CENTER"})
A(KeepTogether([hdr_table, Spacer(1, 4),
                Paragraph("4 + 32 + 32 + 4 + 4 + 4 = <b>80 bytes</b>", Small)]))

# 3
A(Paragraph("3 &nbsp; The fields, one at a time", H1))

A(Paragraph("version &nbsp;(4 bytes)", H2))
A(Paragraph(
    "Originally a plain version number. Now most of its bits are used for soft-fork signalling "
    "(BIP9): miners flip a bit to say &ldquo;I support this proposed rule change.&rdquo; When "
    "enough blocks in a window signal support, the rule activates. A miner can technically roll "
    "a few of these bits as extra search space, but it is noise compared to the nonce.", Body))

A(Paragraph("prev_block_hash &nbsp;(32 bytes)", H2))
A(Paragraph(
    "The double-SHA256 of the previous block&rsquo;s 80-byte header. Completely fixed &mdash; it "
    "is determined by whichever block is currently at the tip of the chain.", Body))
A(Paragraph(
    "This is the field that makes the chain tamper-evident. Because block N+1 commits to the "
    "exact bytes of block N, editing anything in block N changes its hash and leaves block N+1 "
    "pointing at something that no longer exists. Note the direction: the <b>successor</b> "
    "detects the tampering, never the predecessor. A block is protected by what is built on top "
    "of it, not by anything beneath it.", Body))

A(Paragraph("merkle_root &nbsp;(32 bytes)", H2))
A(Paragraph(
    "Every transaction in the block is hashed, then hashes are paired and hashed together, "
    "repeatedly, until one 32-byte value remains. That is the Merkle root.", Body))
A(Paragraph(
    "It is what keeps the header at a constant 80 bytes no matter whether the block holds one "
    "transaction or four thousand. It also means the header commits to the full contents of the "
    "block: change any transaction and the root changes, so the header changes, so the "
    "block&rsquo;s hash changes and its proof of work is destroyed.", Body))
A(Paragraph(
    "<b>This is the field the miner varies indirectly</b>, and section 4 explains how.", Body))

A(Paragraph("timestamp &nbsp;(4 bytes)", H2))
A(Paragraph(
    "Unix seconds, written by the miner. It is only loosely constrained: it must be greater than "
    "the median timestamp of the previous 11 blocks, and no more than 2 hours ahead of "
    "network-adjusted time. Blocks can and do appear slightly out of chronological order.", Body))
A(Paragraph("Ordering in Bitcoin comes from the hash links, never from timestamps.", Body))

A(Paragraph("nBits &nbsp;(4 bytes)", H2))
A(Paragraph(
    "The current target, squeezed into 4 bytes. One byte is an exponent, three bytes are a "
    "mantissa &mdash; essentially scientific notation for a 256-bit number.", Body))
A(Paragraph(
    "It is not an instruction from anyone. Every node independently recomputes what nBits should "
    "be every 2016 blocks (roughly two weeks) by measuring how long those blocks actually took, "
    "and rejects any block whose nBits disagrees.", Body))

A(Paragraph("nonce &nbsp;(4 bytes)", H2))
A(Paragraph(
    "A counter with no meaning. It affects nothing in the protocol and is ignored by every part "
    "of the system except the hash function. It exists purely so that miners have something free "
    "to change.", Body))
A(Paragraph(
    "4 bytes gives 2<super>32</super> values, about 4.3 billion &mdash; which modern hardware "
    "exhausts in well under a second. That is the reason the extranonce exists.", Body))

# 4
A(Paragraph("4 &nbsp; The nonce is not enough: extranonce", H1))
A(Paragraph(
    "Once all 4.3 billion nonce values are used up without a win, the miner needs a fresh header. "
    "It gets one by changing the <b>coinbase transaction</b> &mdash; the special transaction the "
    "miner writes itself to collect the block reward.", Body))
A(Paragraph(
    "The coinbase carries a free-form data area. Consensus allows its signature script to be "
    "<b>2 to 100 bytes</b>; BIP34 reserves the first few for the block height, leaving roughly 96 "
    "bytes the miner can fill with anything. That spare space is the <b>extranonce</b>.", Body))
A(Paragraph(
    "Change one byte of it and the coinbase transaction&rsquo;s hash changes, so the Merkle root "
    "changes, so the header is entirely different &mdash; and 4.3 billion fresh nonces become "
    "available.", Body))

A(datatable(
    ["Field", "Typical size", "Chosen by", "Purpose"],
    [
        [Paragraph("<font face='Courier'>extranonce1</font>", CellB), "~4 bytes", "The pool",
         "Unique per connection, so no two miners in a pool ever grind identical work."],
        [Paragraph("<font face='Courier'>extranonce2</font>", CellB), "~4&ndash;8 bytes",
         "The miner", "The miner&rsquo;s own outer counter."],
    ],
    widths=[30 * mm, 24 * mm, 22 * mm, 89 * mm]))
A(Spacer(1, 8))

A(Paragraph("The search is therefore nested:", Body))
E(codeblock([
    "for extranonce in 0.. {          // EXPENSIVE",
    "    rebuild coinbase transaction",
    "    recompute merkle root        //   -> whole new header",
    "",
    "    for nonce in 0..2^32 {       // CHEAP",
    "        hash 80 bytes, compare to target",
    "    }",
    "}",
]))
A(Paragraph(
    "The asymmetry is the point. Rolling the nonce means rehashing 80 bytes. Rolling the "
    "extranonce means rebuilding a transaction and recomputing the entire Merkle tree. So the "
    "cheap loop goes inside.", Body))
A(Paragraph(
    "Combined, 2<super>32</super> nonces multiplied by roughly 2<super>64</super> extranonce "
    "values gives about 2<super>96</super> distinct headers &mdash; vastly more than will ever be "
    "needed.", Body))

# 5
A(Paragraph("5 &nbsp; Target, difficulty, and &ldquo;below the target&rdquo;", H1))
A(Paragraph(
    "A hash is a 256-bit integer. The target is a 256-bit integer. &ldquo;Below the target&rdquo; "
    "is a plain numeric comparison &mdash; nothing clever.", Body))
A(Paragraph(
    "SHA-256 output is spread uniformly across the whole range, so if the target sits at one "
    "millionth of the maximum, roughly one in a million guesses lands below it. Lower target, "
    "narrower winning band, more work. That is all difficulty is.", Body))
A(Paragraph(
    "Because a small 256-bit number written in fixed-width hex has zeros at the front, this shows "
    "up visually as &ldquo;the hash starts with many zeros.&rdquo; Same statement, easier to "
    "eyeball.", Body))

E(callout("The retarget",
          "Every 2016 blocks, each node measures how long those blocks took and adjusts the "
          "target so the next 2016 average 10 minutes each. Faster than expected means hashpower "
          "joined, so the target drops. Nobody administers this &mdash; every node computes the "
          "same number from the same timestamps and independently reaches the same answer."))

# 6
A(Paragraph("6 &nbsp; Endianness: the trap in Stage 2", H1))
A(Paragraph(
    "The single most common source of &ldquo;my hash does not match.&rdquo; Worth reading twice "
    "before writing the serializer.", Body))
A(datatable(
    ["Field", "Stored in the header as", "Displayed by explorers as"],
    [
        [Paragraph("<font face='Courier'>version, timestamp, nBits, nonce</font>", CellB),
         "Little-endian integers", "Ordinary decimal or hex numbers"],
        [Paragraph("<font face='Courier'>prev_block_hash, merkle_root</font>", CellB),
         "Raw 32 bytes, internal order", "<b>Byte-reversed</b> from the stored order"],
    ],
    widths=[52 * mm, 55 * mm, 58 * mm]))
A(Spacer(1, 8))
A(Paragraph(
    "So a block hash printed as <font face='Courier'>0000...a3f1</font> on a block explorer is "
    "stored in the next block&rsquo;s header with its bytes in the opposite order. The leading "
    "zeros you see when reading are trailing zeros in memory.", Body))
A(Paragraph(
    "Expect to get this wrong three or four times. That is the stage working as intended &mdash; "
    "it is what makes the format physical rather than described.", Body))

# 7
A(Paragraph("7 &nbsp; Quick reference", H1))
A(datatable(
    ["Quantity", "Value"],
    [
        ["Header size", "80 bytes, always"],
        ["Hash function",
         Paragraph("<font face='Courier'>SHA256(SHA256(header))</font> &mdash; applied twice",
                   CellB)],
        ["Nonce space", Paragraph("2<super>32</super> &asymp; 4.3 billion", CellB)],
        ["Extranonce space",
         Paragraph("~2<super>64</super> in practice; up to ~96 free bytes allowed", CellB)],
        ["Coinbase script limit", "2 to 100 bytes (consensus rule)"],
        ["Retarget interval", "2016 blocks, about two weeks"],
        ["Target block time", "10 minutes"],
        ["Max retarget step", "4x up or down, per adjustment"],
        ["Timestamp rules", "&gt; median of previous 11 blocks; &lt; network time + 2 hours"],
    ],
    widths=[46 * mm, 119 * mm]))

A(Spacer(1, 14))
E(rule(0, 6))
A(Paragraph(
    "Next: Stage 1 builds the grinding loop against a toy target. Stage 2 replaces the toy input "
    "with the real 80 bytes described above.", Small))


def decorate(canv, doc):
    canv.saveState()
    canv.setFont("Helvetica", 7.5)
    canv.setFillColor(MUTED)
    canv.drawString(22 * mm, 13 * mm, "Rust Miner  \u00b7  Notes 01")
    canv.drawRightString(A4[0] - 22 * mm, 13 * mm, "%d" % doc.page)
    canv.setStrokeColor(RULE)
    canv.setLineWidth(0.5)
    canv.line(22 * mm, 16.5 * mm, A4[0] - 22 * mm, 16.5 * mm)
    canv.restoreState()


doc = BaseDocTemplate(OUT, pagesize=A4,
                      leftMargin=22 * mm, rightMargin=22 * mm,
                      topMargin=20 * mm, bottomMargin=22 * mm,
                      title="Mining and the Block Header",
                      author="Soroush", subject="Rust Miner project notes")
frame = Frame(doc.leftMargin, doc.bottomMargin, doc.width, doc.height, id="f")
doc.addPageTemplates([PageTemplate(id="main", frames=[frame], onPage=decorate)])
doc.build(story)
print("written:", OUT)
