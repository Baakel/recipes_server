use neo4rs::*;
// use uuid::Uuid;
use crate::models::GraphPool;

pub async fn get_user_from_db (
    graph: GraphPool,
    u_id: &str
) -> Option<Node> {
    let mut res = graph
        .execute(
            query("MATCH (u:User) WHERE u.id = $id RETURN u")
                .param("id", u_id),
        )
        .await
        .expect("Couldn't find that Uuid");

    let row = res.next().await.expect("Couldn't fetch row");
    row.as_ref()?.get::<Node>("u")
}
