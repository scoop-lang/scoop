import CoreFoundation
import CoreText

private func makeFont(_ size: Double) -> CTFont {
    let name = CFStringCreateWithCString(
        nil, "Helvetica", CFStringBuiltInEncodings.UTF8.rawValue
    )!
    return CTFontCreateWithName(name, size, nil)
}

@c(scoop_ct_font_size)
public func fontSize(_ size: Double) -> Double {
    return CTFontGetSize(makeFont(size))
}

@c(scoop_ct_glyph_count)
public func glyphCount() -> Int32 {
    return Int32(CTFontGetGlyphCount(makeFont(12)))
}
