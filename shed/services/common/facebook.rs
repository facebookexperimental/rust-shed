// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use crate::FbStatus;

impl From<FbStatus> for fb303::fb_status {
    fn from(status: FbStatus) -> Self {
        match status {
            FbStatus::Dead => fb303::fb_status::DEAD,
            FbStatus::Starting => fb303::fb_status::STARTING,
            FbStatus::Alive => fb303::fb_status::ALIVE,
            FbStatus::Stopping => fb303::fb_status::STOPPING,
            FbStatus::Stopped => fb303::fb_status::STOPPED,
            FbStatus::Warning => fb303::fb_status::WARNING,
        }
    }
}
