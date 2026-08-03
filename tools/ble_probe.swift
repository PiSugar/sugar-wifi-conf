import CoreBluetooth
import Foundation
import Darwin

let serviceUUID = CBUUID(string: "FD2B4448-AA0F-4A15-A62F-EB0BE77A0000")
final class Probe: NSObject, CBCentralManagerDelegate, CBPeripheralDelegate {
    private var central: CBCentralManager!
    private var peripheral: CBPeripheral?
    private var timeout: DispatchWorkItem?
    private var startupTimeout: DispatchWorkItem?

    override init() {
        super.init()
        central = CBCentralManager(delegate: self, queue: .main)
    }

    func setStartupTimeout(_ item: DispatchWorkItem) {
        startupTimeout = item
    }

    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        startupTimeout?.cancel()
        print("central state: \(stateName(central.state))")
        guard central.state == .poweredOn else { return }

        print("scanning for \(serviceUUID.uuidString) ...")
        central.scanForPeripherals(withServices: [serviceUUID], options: [
            CBCentralManagerScanOptionAllowDuplicatesKey: true
        ])

        let item = DispatchWorkItem {
            print("scan timeout: no matching peripheral discovered")
            exit(2)
        }
        timeout = item
        DispatchQueue.main.asyncAfter(deadline: .now() + 20, execute: item)
    }

    func centralManager(_ central: CBCentralManager,
                        didDiscover peripheral: CBPeripheral,
                        advertisementData: [String: Any],
                        rssi RSSI: NSNumber) {
        let name = peripheral.name ?? advertisementData[CBAdvertisementDataLocalNameKey] as? String ?? "(no name)"
        print("discovered: name=\(name) id=\(peripheral.identifier.uuidString) rssi=\(RSSI)")
        print("advertisement: \(advertisementData)")

        timeout?.cancel()
        central.stopScan()
        self.peripheral = peripheral
        peripheral.delegate = self
        print("connecting ...")
        central.connect(peripheral, options: nil)
    }

    func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        print("connected")
        peripheral.discoverServices([serviceUUID])
    }

    func centralManager(_ central: CBCentralManager,
                        didFailToConnect peripheral: CBPeripheral,
                        error: Error?) {
        print("connect failed: \(error?.localizedDescription ?? "unknown error")")
        exit(3)
    }

    func centralManager(_ central: CBCentralManager,
                        didDisconnectPeripheral peripheral: CBPeripheral,
                        error: Error?) {
        print("disconnected: \(error?.localizedDescription ?? "no error")")
        exit(error == nil ? 0 : 4)
    }

    func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        if let error {
            print("discover services failed: \(error.localizedDescription)")
            exit(5)
        }

        let services = peripheral.services ?? []
        print("services: \(services.map { $0.uuid.uuidString })")
        guard !services.isEmpty else { exit(6) }
        for service in services {
            peripheral.discoverCharacteristics(nil, for: service)
        }
    }

    func peripheral(_ peripheral: CBPeripheral,
                    didDiscoverCharacteristicsFor service: CBService,
                    error: Error?) {
        if let error {
            print("discover characteristics failed for \(service.uuid): \(error.localizedDescription)")
            exit(7)
        }

        let characteristics = service.characteristics ?? []
        print("characteristics for \(service.uuid.uuidString):")
        for ch in characteristics {
            print("  \(ch.uuid.uuidString) props=\(ch.properties)")
            if ch.properties.contains(.read) {
                peripheral.readValue(for: ch)
            }
        }

        DispatchQueue.main.asyncAfter(deadline: .now() + 5) {
            print("probe complete")
            self.central.cancelPeripheralConnection(peripheral)
        }
    }

    func peripheral(_ peripheral: CBPeripheral,
                    didUpdateValueFor characteristic: CBCharacteristic,
                    error: Error?) {
        if let error {
            print("read \(characteristic.uuid.uuidString) failed: \(error.localizedDescription)")
            return
        }

        let data = characteristic.value ?? Data()
        let text = String(data: data, encoding: .utf8) ?? data.map { String(format: "%02x", $0) }.joined()
        print("read \(characteristic.uuid.uuidString): \(text)")
    }

    private func stateName(_ state: CBManagerState) -> String {
        switch state {
        case .unknown: return "unknown"
        case .resetting: return "resetting"
        case .unsupported: return "unsupported"
        case .unauthorized: return "unauthorized"
        case .poweredOff: return "poweredOff"
        case .poweredOn: return "poweredOn"
        @unknown default: return "future"
        }
    }
}

let probe = Probe()
setbuf(stdout, nil)
let startupTimeout = DispatchWorkItem {
    print("startup timeout: CoreBluetooth did not report central state")
}
probe.setStartupTimeout(startupTimeout)
DispatchQueue.main.asyncAfter(deadline: .now() + 10, execute: startupTimeout)
RunLoop.main.run()
