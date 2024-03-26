inspect_asm::alloc_u32::try_down_a:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	add rax, -4
	cmp rax, qword ptr [rcx + 8]
	jb .LBB_3
	mov qword ptr [rcx], rax
.LBB_2:
	mov dword ptr [rax], esi
	pop rbx
	ret
.LBB_3:
	mov ebx, esi
	call bump_scope::bump_scope::BumpScope<_,_,A>::do_alloc_sized_in_another_chunk
	mov esi, ebx
	test rax, rax
	jne .LBB_2
	xor eax, eax
	pop rbx
	ret