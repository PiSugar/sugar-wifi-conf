let util = require('util')
let bleno = require('bleno')
let UUID = require('../sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic
let BlenoDescriptor = bleno.Descriptor

let ServiceNameCharacteristic = function() {
  ServiceNameCharacteristic.super_.call(this, {
    uuid: UUID.SERVICE_NAME,
    properties: ['read'],
    value: new Buffer('PiSugar BLE Wifi Config'),
    descriptors: [
      new BlenoDescriptor({
        uuid: '2001',
        value: 'PiSugar BLE Wifi Config'
      })
    ]
  })
}
util.inherits(ServiceNameCharacteristic, BlenoCharacteristic)

module.exports = ServiceNameCharacteristic