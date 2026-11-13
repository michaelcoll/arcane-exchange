import CoreGraphics
import SwiftUI
import Testing

@testable import ArcaneExchangeCore

struct TextStyleSizeTests {
    @Test(arguments: [
        (Font.TextStyle.caption2, CGFloat(11)), (.caption, CGFloat(12)), (.footnote, CGFloat(13)),
        (.subheadline, CGFloat(15)), (.callout, CGFloat(16)), (.body, CGFloat(17)), (.title, CGFloat(17))
    ])
    func standardSizeFollowsTheTextStyle(style: Font.TextStyle, size: CGFloat) {
        #expect(TextStyleSize.standard(of: style) == size)
    }
}
