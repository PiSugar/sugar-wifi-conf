let util = require('util')
let bleno = require('@stoprocent/bleno')
let UUID = require('../sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic
let BlenoDescriptor = bleno.Descriptor

module.exports = new BlenoCharacteristic({
  uuid: UUID.SERVICE_NAME,
  properties: ['read'],
  value: Buffer.from('PiSugar BLE Wifi Config'),
  descriptors: [
    new BlenoDescriptor({
      uuid: '2001',
      value: 'PiSugar BLE Wifi Config'
    })
  ]
})