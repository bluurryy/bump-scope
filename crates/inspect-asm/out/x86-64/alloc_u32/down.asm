inspect_asm::alloc_u32::down:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	and rax, -4
	add rax, -4
	cmp rax, qword ptr [rcx + 8]
	jb .LBB0_0
	mov qword ptr [rcx], rax
	test rax, rax
	je .LBB0_0
	mov dword ptr [rax], esi
	pop rbx
	ret
.LBB0_0:
	mov ebx, esi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov esi, ebx
	mov dword ptr [rax], esi
	pop rbx
	ret
