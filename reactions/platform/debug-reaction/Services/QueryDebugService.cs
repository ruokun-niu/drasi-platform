// Copyright 2024 The Drasi Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using Dapr;
using Dapr.Actors;
using Dapr.Actors.Client;
using Dapr.Client;

using System.Net.WebSockets;
using Drasi.Reactions.Debug.Server.Models;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;
using Drasi.Reaction.SDK.Models.QueryOutput;
using Drasi.Reaction.SDK.Services;
using System.Text.Json;
using System.Text;

namespace Drasi.Reactions.Debug.Server.Services
{
	public class QueryDebugService : IQueryDebugService
	{
		private readonly IResultViewClient _queryApi;
		private readonly IActorProxyFactory _actorProxyFactory;
		private readonly DaprClient _daprClient;

		private readonly ILogger<QueryDebugService> _logger;

		private List<string> _activeQueries = new();

		private readonly LinkedList<JsonElement> _rawEvents = new();

		private readonly IChangeBroadcaster _webSocketService;

		private readonly IManagementClient _managementClient;

		public QueryDebugService(IResultViewClient queryApi, IActorProxyFactory actorProxyFactory, DaprClient daprClient, IChangeBroadcaster webSocketService, ILogger<QueryDebugService> logger, IManagementClient managementClient)
		{
			_logger = logger;
			_daprClient = daprClient;
			_queryApi = queryApi;
			_webSocketService = webSocketService;
			_actorProxyFactory = actorProxyFactory;
			_managementClient = managementClient;
		}


		public async Task<LinkedList<JsonElement>> GetRawEvents()
		{
			return _rawEvents;
		}

		public void SetActiveQueries(IEnumerable<string> queries)
		{
			_activeQueries = queries.ToList();
		}
		public async Task<Dictionary<string, object>> GetDebugInfo(string queryId)
		{
			try
			{
				var queryContainerId = await _managementClient.GetQueryContainerId(queryId);
				var actor = _actorProxyFactory.Create(new ActorId(queryId), $"{queryContainerId}.ContinuousQuery");
				return await actor.InvokeMethodAsync<Dictionary<string, object>>("getStatus");
			}
			catch (Exception ex)
			{
				return new Dictionary<string, object>
				{
					{ "error", ex.Message }
				};
			}
		}

		public async Task<QueryResult> GetQueryResult(string queryId)
		{
			var queryResult = await InitResult(queryId);
			return queryResult;
		}

		public async Task ProcessChange(ChangeEvent change)
		{
			var jsonEvent = JsonSerializer.Deserialize<JsonElement>(change.ToJson());
			await ProcessRawChange(change);
			await _webSocketService.BroadcastToStream("stream", jsonEvent);
			
		}
		public async Task ProcessRawChange(ChangeEvent change)
		{
			var queryId = change.QueryId;
			if (!_activeQueries.Contains(queryId))
				return;

			var queryResult = new QueryResult();
			foreach (var item in change.DeletedResults)
			{
				var result = JsonSerializer.Deserialize<JsonElement>(JsonSerializer.Serialize(item));
				queryResult.DeletedResults.Add(result);
			}
			foreach (var item in change.AddedResults)
			{
				var result = JsonSerializer.Deserialize<JsonElement>(JsonSerializer.Serialize(item));
				queryResult.AddedResults.Add(result);
			}
			foreach (var item in change.UpdatedResults)
			{
				var result = JsonSerializer.Deserialize<JsonElement>(JsonSerializer.Serialize(item));
				queryResult.UpdatedResults.Add(result);
			}

			await _webSocketService.BroadcastToQueryId(queryId, queryResult);
		}

		public async Task ProcessControlSignal(ControlEvent change)
		{
			var queryId = change.QueryId;
			if (!_activeQueries.Contains(queryId))
				return;

			// TODO: update queryResult based on control signal
			switch (change.ControlSignal.Kind)
			{
				case ControlSignalKind.Deleted:
					var queryResult = new QueryResult();
					queryResult.ResultsClear = true;
					await _webSocketService.BroadcastToQueryId(queryId, queryResult);
					break;
				case ControlSignalKind.BootstrapStarted:
					queryResult = new QueryResult();
					queryResult.ResultsClear = true;
					await _webSocketService.BroadcastToQueryId(queryId, queryResult);
					break;
			}

			var jsonEvent = JsonSerializer.Deserialize<JsonElement>(change.ToJson());
			await _webSocketService.BroadcastToStream("stream", jsonEvent);
		}


		private async Task<QueryResult> InitResult(string queryId)
		{
			_logger.LogInformation("Initializing query: " + queryId);
			var result = new QueryResult();
			try
			{
				await foreach (var item in _queryApi.GetCurrentResult(queryId))
				{
					if (item == null)
					{
						_logger.LogWarning("Received null item from GetCurrentResult for queryId: {QueryId}", queryId);
						continue;
					}

					var data = item.Data;
					if (data == null)
					{
						_logger.LogWarning("Item.Data is null for queryId: {QueryId}", queryId);
						continue;
					}
					var jsonElement = JsonSerializer.Deserialize<JsonElement>(JsonSerializer.Serialize(data));
					result.AddedResults.Add(jsonElement);


				}
			}
			catch (Exception ex)
			{
				result.Errors.Add("Error fetching initial data: " + ex.Message);
				_logger.LogError(ex, "Error fetching initial data: " + ex.Message);
			}

			return result;
		}
	}


}
