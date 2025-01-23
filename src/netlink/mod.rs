use futures_util::TryStreamExt;
use rtnetlink::packet_route::address::AddressMessage;
use rtnetlink::packet_route::link::LinkMessage;
use rtnetlink::packet_route::route::RouteAddress;
use rtnetlink::Error as RtNetError;
use rtnetlink::Handle;
use rtnetlink::LinkUnspec;
use rtnetlink::RouteMessageBuilder;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use tracing::info;

use crate::config::CONTAINER_IPV4_ADDR;
const LINK_NAME: &str = "dummy0";

pub async fn init_local_link() -> Result<(), anyhow::Error> {
    let (conn, handle, _) = rtnetlink::new_connection()?;
    tokio::spawn(conn);

    info!("ensuring link {} exists", LINK_NAME);
    let link = ensure_dummy_link(&handle).await?;

    info!(
        "ensuring {} present on link {}",
        CONTAINER_IPV4_ADDR, LINK_NAME
    );
    ensure_dummy_addr(&handle, &link).await?;

    info!("ensuring link {} is up", LINK_NAME);
    ensure_dummy_link_up(&handle, &link).await?;

    info!("ensuring route to {}", CONTAINER_IPV4_ADDR);
    ensure_route(&handle).await?;
    Ok(())
}

async fn ensure_dummy_link(handle: &Handle) -> Result<LinkMessage, RtNetError> {
    let mut links = handle.link().get().match_name(LINK_NAME.into()).execute();
    if let Ok(Some(link)) = links.try_next().await {
        return Ok(link);
    }
    handle
        .link()
        .add(rtnetlink::LinkDummy::new(LINK_NAME).build())
        .execute()
        .await?;
    let mut links = handle.link().get().match_name(LINK_NAME.into()).execute();
    match links.try_next().await? {
        Some(link) => Ok(link),
        // TODO: fix this error
        None => Err(RtNetError::NamespaceError("link not found".to_string())),
    }
}
async fn ensure_dummy_addr(handle: &Handle, link: &LinkMessage) -> Result<(), RtNetError> {
    if let Some(addr) = handle
        .address()
        .get()
        .set_link_index_filter(link.header.index)
        .execute()
        .try_next()
        .await?
    {
        if addr_matches(&addr) {
            return Ok(());
        }
    }
    handle
        .address()
        .add(link.header.index, IpAddr::V4(CONTAINER_IPV4_ADDR), 32)
        .execute()
        .await
}

async fn ensure_dummy_link_up(handle: &Handle, link: &LinkMessage) -> Result<(), RtNetError> {
    handle
        .link()
        .set(LinkUnspec::new_with_index(link.header.index).up().build())
        .execute()
        .await
}

async fn ensure_route(handle: &Handle) -> Result<(), RtNetError> {
    let mut routes = handle
        .route()
        .get(RouteMessageBuilder::<Ipv4Addr>::new().build())
        .execute();

    while let Some(route) = routes.try_next().await? {
        if route.attributes.iter().any(|r| match r {
            rtnetlink::packet_route::route::RouteAttribute::Destination(route_address) => {
                *route_address == RouteAddress::Inet(CONTAINER_IPV4_ADDR)
            }
            _ => false,
        }) {
            return Ok(());
        }
    }

    let route = RouteMessageBuilder::<Ipv4Addr>::new()
        .destination_prefix(CONTAINER_IPV4_ADDR, 32)
        .gateway(CONTAINER_IPV4_ADDR)
        .build();
    handle.route().add(route).execute().await
}

fn addr_matches(addr_message: &AddressMessage) -> bool {
    addr_message.attributes.iter().any(|attr| match attr {
        rtnetlink::packet_route::address::AddressAttribute::Address(ip_addr) => {
            *ip_addr == CONTAINER_IPV4_ADDR
        }
        _ => false,
    })
}
