/// How the cards of the showcase are spread over the columns of the wall.
enum ShowcaseWallLayout {
    /// `items` dealt out in order over `count` columns, each at least `minimumPerColumn` long:
    /// a showcase too small for that repeats its cards. No column at all without items.
    static func columns<Item>(_ items: [Item], count: Int, minimumPerColumn: Int) -> [[Item]] {
        guard !items.isEmpty, count > 0 else { return [] }
        let length = max(items.count, count * minimumPerColumn)
        var columns = [[Item]](repeating: [], count: count)
        for index in 0 ..< length {
            columns[index % count].append(items[index % items.count])
        }
        return columns
    }
}
