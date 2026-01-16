/**
 * Test fixtures - sample data for tests
 */

import type {
  UserResponse,
  AppStatus,
  TokenResponse,
  WorkItem,
  WorkItemWithChildren,
  PaginatedResponse,
  WorkItemStatsResponse,
  PersonalReport,
  TempoReport,
  AnalyzeResponse,
} from '@/types'

// Auth fixtures
export const mockUser: UserResponse = {
  id: 'user-123',
  email: 'test@example.com',
  name: 'Test User',
  username: 'testuser',
  is_active: true,
  is_admin: false,
  created_at: '2024-01-01T00:00:00Z',
}

export const mockAppStatus: AppStatus = {
  has_users: true,
  user_count: 1,
  first_user: mockUser,
  local_mode: true,
}

export const mockTokenResponse: TokenResponse = {
  access_token: 'mock-jwt-token-12345',
  token_type: 'bearer',
}

// Work Items fixtures
export const mockWorkItem: WorkItem = {
  id: 'work-item-1',
  user_id: 'user-123',
  source: 'git',
  source_id: 'commit-abc123',
  title: 'Fix authentication bug',
  description: 'Fixed JWT token validation',
  hours: 2.5,
  date: '2024-01-15',
  jira_issue_key: 'PROJ-123',
  category: 'development',
  synced_to_tempo: false,
  created_at: '2024-01-15T10:00:00Z',
  updated_at: '2024-01-15T10:00:00Z',
}

export const mockWorkItemWithChildren: WorkItemWithChildren = {
  ...mockWorkItem,
  child_count: 3,
}

export const mockWorkItems: WorkItemWithChildren[] = [
  mockWorkItemWithChildren,
  {
    id: 'work-item-2',
    user_id: 'user-123',
    source: 'claude',
    title: 'Implement new feature',
    hours: 4.0,
    date: '2024-01-15',
    synced_to_tempo: true,
    created_at: '2024-01-15T14:00:00Z',
    updated_at: '2024-01-15T14:00:00Z',
    child_count: 0,
  },
]

export const mockPaginatedWorkItems: PaginatedResponse<WorkItemWithChildren> = {
  items: mockWorkItems,
  total: 2,
  page: 1,
  per_page: 20,
  pages: 1,
}

export const mockWorkItemStats: WorkItemStatsResponse = {
  total_items: 10,
  total_hours: 45.5,
  hours_by_source: {
    git: 20.0,
    claude: 15.5,
    gitlab: 10.0,
  },
  hours_by_project: {
    'project-a': 25.0,
    'project-b': 20.5,
  },
  hours_by_category: {
    development: 30.0,
    review: 10.0,
    meeting: 5.5,
  },
  daily_hours: [
    { date: '2024-01-15', hours: 8.0, count: 3 },
    { date: '2024-01-14', hours: 7.5, count: 2 },
  ],
  jira_mapping: {
    mapped: 8,
    unmapped: 2,
    percentage: 80,
  },
  tempo_sync: {
    synced: 6,
    not_synced: 4,
    percentage: 60,
  },
}

// Reports fixtures
export const mockPersonalReport: PersonalReport = {
  start_date: '2024-01-01',
  end_date: '2024-01-15',
  total_hours: 45.5,
  total_items: 10,
  items_by_date: [
    { date: '2024-01-15', hours: 8.0, count: 3 },
    { date: '2024-01-14', hours: 7.5, count: 2 },
  ],
  work_items: [mockWorkItem],
}

export const mockTempoReport: TempoReport = {
  period: 'weekly',
  start_date: '2024-01-08',
  end_date: '2024-01-14',
  total_hours: 40.0,
  total_items: 8,
  projects: [
    {
      project: 'Project A',
      hours: 25.0,
      item_count: 5,
      summaries: ['Implemented authentication', 'Fixed bugs'],
    },
    {
      project: 'Project B',
      hours: 15.0,
      item_count: 3,
      summaries: ['Code review', 'Documentation'],
    },
  ],
  used_llm: true,
}

export const mockAnalyzeResponse: AnalyzeResponse = {
  start_date: '2024-01-08',
  end_date: '2024-01-14',
  total_minutes: 2400,
  total_hours: 40.0,
  dates_covered: ['2024-01-08', '2024-01-09', '2024-01-10', '2024-01-11', '2024-01-12'],
  projects: [
    {
      project_name: 'Project A',
      project_path: '/path/to/project-a',
      total_minutes: 1500,
      total_hours: 25.0,
      daily_entries: [
        {
          date: '2024-01-08',
          minutes: 480,
          hours: 8.0,
          todos: ['Fix bug'],
          summaries: ['Fixed authentication bug'],
          description: 'Authentication work',
        },
      ],
      jira_id: 'PROJ-123',
      jira_id_suggestions: ['PROJ-123', 'PROJ-124'],
    },
  ],
  mode: 'git',
}
