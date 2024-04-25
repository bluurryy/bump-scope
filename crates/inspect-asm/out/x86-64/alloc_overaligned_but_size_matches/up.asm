inspect_asm::alloc_overaligned_but_size_matches::up:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, qword ptr [rcx + 8]
	sub rdx, rax
	cmp rdx, 4
	jb .LBB_2
	lea rdx, [rax + 7]
	and rdx, -4
	mov qword ptr [rcx], rdx
	mov dword ptr [rax], esi
	pop rbx
	ret
.LBB_2:
	mov ebx, esi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov esi, ebx
	mov dword ptr [rax], esi
	pop rbx
	ret