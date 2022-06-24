// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, Endpoint},
  server::{
    ButtplugServerResultFuture,
    device::{
      protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
      configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder, ProtocolAttributesIdentifier},
      hardware::{Hardware, HardwareWriteCmd},
    },
  },
};
use std::sync::Arc;

super::default_protocol_definition!(Patoo, "patoo");

#[derive(Default, Debug)]
pub struct PatooFactory {}

impl ButtplugProtocolFactory for PatooFactory {
  fn try_create(
    &self,
    hardware: Arc<Hardware>,
    builder: ProtocolDeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    // Patoo Love devices have wildcarded names of ([A-Z]+)\d*
    // Force the identifier lookup to the non-numeric portion
    let c: Vec<char> = hardware.name().chars().collect();
    let mut i = 0;
    while i < c.len() && !c[i].is_digit(10) {
      i += 1;
    }
    let name: String = c[0..i].iter().collect();
    Box::pin(async move {
      let device_attributes = builder.create(hardware.address(), &ProtocolAttributesIdentifier::Identifier(name), &hardware.endpoints())?;
      Ok(Box::new(Patoo::new(device_attributes)) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self) -> &'static str {
    "patoo"
  }
}

impl ProtocolHandler for Patoo {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, true)?;
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        // Default to vibes
        let mut mode: u8 = 4u8;

        // Use vibe 1 as speed
        let mut speed = cmds[0].unwrap_or(0) as u8;
        if speed == 0 {
          mode = 0;

          // If we have a second vibe and it's not also 0, use that
          if cmds.len() > 1 {
            speed = cmds[1].unwrap_or(0) as u8;
            if speed != 0 {
              mode |= 0x80;
            }
          }
        } else if cmds.len() > 1 && cmds[1].unwrap_or(0) as u8 != 0 {
          // Enable second vibe if it's not at 0
          mode |= 0x80;
        }

        fut_vec.push(device.write_value(HardwareWriteCmd::new(Endpoint::Tx, vec![speed], true)));
        fut_vec.push(device.write_value(HardwareWriteCmd::new(Endpoint::TxMode, vec![mode], true)));
      }

      // TODO Just use join_all here
      for fut in fut_vec {
        // TODO Do something about possible errors here
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{Endpoint, StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      hardware::communication::test::{
        check_test_recv_empty,
        check_test_recv_value,
        new_bluetoothle_test_device,
      },
    },
    util::async_manager,
  };

  #[test]
  pub fn test_patoo_protocol_devil() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("PBT821")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      let command_receiver_txmode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // We just vibe 1 so expect 2 writes (mode 0x04)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![50], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x04], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // no-op
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // setting second vibe whilst changing vibe 1, 2 writes (mode 1)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![10], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x84], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.9),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // only vibe 1 changed, 2 writes, same data
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![10], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x84], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.0),
              VibrateSubcommand::new(1, 0.9),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // turn off vibe 1, 2 writes (mode 0x80)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![90], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x80], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      // stop on both, 2 writes (mode 0)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));
    });
  }

  #[test]
  pub fn test_patoo_protocol_carrot() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("PTVEA2601")
        .await
        .expect("Test, assuming infallible");

      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      let command_receiver_txmode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // We just vibe 1 so expect 2 writes (mode 0x04)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![50], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x04], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // no-op
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      assert!(device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .is_err());
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      // stop on both, 2 writes (mode 0)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));
    });
  }
}
