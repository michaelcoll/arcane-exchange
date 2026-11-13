import CoreGraphics
import SwiftUI

/// The point size of a text style. Pure logic: kept off the views.
package enum TextStyleSize {
    /// The size of the text style at the default Dynamic Type setting.
    package static func standard(of style: Font.TextStyle) -> CGFloat {
        switch style {
        case .caption2: 11
        case .caption: 12
        case .footnote: 13
        case .subheadline: 15
        case .callout: 16
        default: 17
        }
    }
}
