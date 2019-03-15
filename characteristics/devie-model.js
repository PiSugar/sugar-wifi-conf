let util = require('util')
let bleno = require('bleno')
let UUID = require('../sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic
let BlenoDescriptor = bleno.Descriptor


let DeviceModelCharacteristic = function() {
  DeviceModelCharacteristic.super_.call(this, {
    uuid: UUID.DEVICE_MODEL,
    properties: ['read'],
    value: new Buffer('Raspberry Pi 3B+'),
    descriptors: [
      new BlenoDescriptor({
        uuid: '2002',
        value: 'Raspberry Hardware Model'
      })
    ]
  })
}
util.inherits(DeviceModelCharacteristic, BlenoCharacteristic)

module.exports = DeviceModelCharacteristic
