import Foundation

/// Where the scan batch is kept on the iPhone, so it survives the app being closed. The view model
/// saves after every change and reads at launch; tests inject an in-memory one.
package protocol ScanBatchStorage {
    func load() -> ScanBatch?
    func save(_ batch: ScanBatch)
}

/// The batch in `UserDefaults`, through `DeviceMemory`: it belongs to the player, so sign-out
/// forgets it.
package struct DeviceScanBatchStorage: ScanBatchStorage {
    private let memory: DeviceMemory

    package init(defaults: UserDefaults = .standard) {
        memory = DeviceMemory(defaults: defaults)
    }

    package func load() -> ScanBatch? {
        memory.read(ScanBatch.self, for: PlayerKey.scanBatch)
    }

    package func save(_ batch: ScanBatch) {
        memory.write(batch, for: PlayerKey.scanBatch)
    }
}
